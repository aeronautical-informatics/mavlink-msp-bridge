use std::clone::Clone;
use std::convert::TryFrom;
use std::io;

use bytes::{Buf, BufMut, BytesMut, IntoBuf};
use crc_any::CRC;
use tokio::codec::{Decoder, Encoder};

/// Request: Master to Slave (`<`)
/// Response: Slave to Master (`>`)
/// Error: Master to Slave or Slave to Master (`!`)
#[derive(Clone, Debug, PartialEq)]
pub enum MSPDirection {
    Request,
    Response,
    Error,
}

impl From<&MSPDirection> for u8 {
    fn from(d: &MSPDirection) -> Self {
        match d {
            MSPDirection::Request => '<' as u8,
            MSPDirection::Response => '>' as u8,
            MSPDirection::Error => '!' as u8,
        }
    }
}

impl TryFrom<u8> for MSPDirection {
    type Error = io::Error;
    fn try_from(byte: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match byte as char {
            '<' => Ok(MSPDirection::Request),
            '>' => Ok(MSPDirection::Response),
            '!' => Ok(MSPDirection::Error),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown MSP direction",
            )),
        }
    }
}

/// V1: (`M`)
/// V2: (`X`)
#[derive(Clone, Debug, PartialEq)]
pub enum MSPVersion {
    V1,
    V2,
}

impl From<&MSPVersion> for u8 {
    fn from(d: &MSPVersion) -> Self {
        match d {
            MSPVersion::V1 => 'M' as u8,
            MSPVersion::V2 => 'X' as u8,
        }
    }
}

impl TryFrom<u8> for MSPVersion {
    type Error = io::Error;
    fn try_from(byte: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match byte as char {
            'M' => Ok(MSPVersion::V1),
            'X' => Ok(MSPVersion::V2),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown MSP version",
            )),
        }
    }
}

/// A flag may only be `Some(_)` if `version == MSPVersion::V2`
#[derive(Clone, Debug, PartialEq)]
pub struct MSPMessage {
    pub version: MSPVersion,
    pub direction: MSPDirection,
    pub flag: Option<u8>,
    pub function: u16,
    pub payload: Vec<u8>,
}

impl std::fmt::Display for MSPMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "({:?} {:?} {:?}\t{:?})",
            self.direction, self.flag, self.function, self.payload
        )
    }
}

impl MSPMessage {
    pub fn checksum(&self) -> u8 {
        match self.version {
            MSPVersion::V1 => {
                panic!("not yet implemented");
            }
            MSPVersion::V2 => {
                let mut crc = CRC::create_crc(0xd5, 8, 0x0, 0x0, false);
                crc.digest(&[self.flag.unwrap_or(0)]);
                crc.digest(&self.function.to_le_bytes());
                let payload_size = u16::try_from(self.payload.len()).expect("payload too big");
                crc.digest(&payload_size.to_le_bytes());
                crc.digest(&self.payload);
                u8::try_from(crc.get_crc()).unwrap()
            }
        }
    }
}

#[derive(Debug)]
enum CodecStep {
    Header,
    V1Fields,
    V2Fields,
    Jumbo,
    Payload,
    Checksum,
}

#[derive(Debug)]
pub struct MSPCodec {
    message: MSPMessage,
    next_step: Option<CodecStep>,
    payload_size: usize,
}

impl MSPCodec {
    pub fn new() -> MSPCodec {
        MSPCodec {
            message: MSPMessage {
                version: MSPVersion::V1,
                direction: MSPDirection::Error,
                flag: None,
                function: 0,
                payload: Vec::new(),
            },
            next_step: None,
            payload_size: 0,
        }
    }
}

impl Decoder for MSPCodec {
    type Item = MSPMessage;
    type Error = io::Error;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match self.next_step {
                Some(CodecStep::Header) if src.len() >= 2 => {
                    self.next_step = None;
                    let mut buf = src.split_to(2).freeze().into_buf();
                    self.message.version = MSPVersion::try_from(buf.get_u8())?;
                    self.message.direction = MSPDirection::try_from(buf.get_u8())?;
                    self.next_step = match self.message.version {
                        MSPVersion::V1 => Some(CodecStep::V1Fields),
                        MSPVersion::V2 => Some(CodecStep::V2Fields),
                    };
                }
                Some(CodecStep::V1Fields) if src.len() >= 2 => {
                    let mut buf = src.split_to(2).freeze().into_buf();
                    self.message.flag = None;
                    self.payload_size = buf.get_u8() as usize;
                    self.message.function = buf.get_u8() as u16;
                    self.next_step = match self.payload_size {
                        255 => Some(CodecStep::Jumbo),
                        _ => Some(CodecStep::Payload),
                    };
                }
                Some(CodecStep::V2Fields) if 5 <= src.len() => {
                    let mut buf = src.split_to(5).freeze().into_buf();
                    self.message.flag = Some(buf.get_u8());
                    self.message.function = buf.get_u16_le();
                    self.payload_size = buf.get_u16_le() as usize;
                    self.next_step = Some(CodecStep::Payload);
                }
                Some(CodecStep::Jumbo) if src.len() >= 2 => {
                    let mut buf = src.split_to(2).freeze().into_buf();
                    self.payload_size = buf.get_u16_le() as usize;
                    self.next_step = Some(CodecStep::Payload);
                }
                Some(CodecStep::Payload) if self.payload_size <= src.len() => {
                    self.message.payload = src.split_to(self.payload_size).to_vec();
                    self.next_step = Some(CodecStep::Checksum);
                    assert_eq!(self.payload_size, self.message.payload.len());
                }
                Some(CodecStep::Checksum) if 1 <= src.len() => {
                    self.next_step = None;
                    let mut buf = src.split_to(1).freeze().into_buf();
                    match self.message.checksum() == buf.get_u8() {
                        true => return Ok(Some(self.message.clone())),
                        false => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "wrong MSP checksum",
                            ))
                        }
                    }
                }
                None if 1 <= src.len() => {
                    if let Some(next_sof) = src.iter().position(|b| *b == b'$') {
                        src.advance(next_sof + 1);
                        self.next_step = Some(CodecStep::Header);
                    } else {
                        src.clear();
                    }
                }
                _ => return Ok(None),
            }
        }
    }
}

impl Encoder for MSPCodec {
    type Item = MSPMessage;
    type Error = io::Error;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item.version {
            MSPVersion::V1 => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown message direction",
            )),
            MSPVersion::V2 => {
                dst.reserve(9 + item.payload.len());
                dst.put("$X");
                dst.put(u8::from(&item.direction));
                dst.put(item.flag.unwrap_or(0));
                dst.put(&item.function.to_le_bytes()[..]);
                let payload_size = u16::try_from(item.payload.len()).expect("payload too big");
                dst.put(&payload_size.to_le_bytes()[..]);
                dst.put(&item.payload);
                dst.put(item.checksum());
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pure_bytes_to_mspv2() {
        let mut codec = MSPCodec::new();
        let mut buf = BytesMut::from(vec![
            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
        ]);

        let expected = MSPMessage {
            version: MSPVersion::V2,
            direction: MSPDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: "Hello flying world".as_bytes().to_vec(),
        };
        let result = codec.decode(&mut buf);
        assert_eq!(expected, result.unwrap().unwrap());
    }

    #[test]
    fn pure_bytes_to_multiple_mspv2() {
        let mut codec = MSPCodec::new();
        let mut buf = BytesMut::from(vec![
            0x24u8, 0x58, 0x3c, 0x00, 0x64, 0x00, 0x00, 0x00, 0x8f, 0x24, 0x58, 0x3e, 0xa5, 0x42,
            0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e,
            0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
        ]);

        let message_1 = MSPMessage {
            version: MSPVersion::V2,
            direction: MSPDirection::Request,
            flag: Some(0),
            function: 100,
            payload: vec![0u8; 0],
        };
        let message_2 = MSPMessage {
            version: MSPVersion::V2,
            direction: MSPDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: "Hello flying world".as_bytes().to_vec(),
        };
        let result = codec.decode(&mut buf);
        assert_eq!(message_1, result.unwrap().unwrap());
        let result = codec.decode(&mut buf);
        assert_eq!(message_2, result.unwrap().unwrap());
    }

    #[test]
    fn noised_bytes_to_mspv2() {
        let mut codec = MSPCodec::new();
        let mut buf = BytesMut::from(vec![
            0x30, 0x60, 0x13, 0x24, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c,
            0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c,
            0x64, 0x82, 0x25,
        ]);

        let expected = MSPMessage {
            version: MSPVersion::V2,
            direction: MSPDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: "Hello flying world".as_bytes().to_vec(),
        };
        let result = codec.decode(&mut buf);
        assert_eq!(expected, result.unwrap().unwrap());
    }

    #[test]
    fn pure_bytes_to_mspv2_partial() {
        let mut codec = MSPCodec::new();
        let expected = MSPMessage {
            version: MSPVersion::V2,
            direction: MSPDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: "Hello flying world".as_bytes().to_vec(),
        };

        let bytes = vec![
            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
        ];

        for n in 0..bytes.len() {
            let mut buf = BytesMut::with_capacity(bytes.len());
            buf.extend_from_slice(&bytes[..n]);
            let result = codec.decode(&mut buf).unwrap();
            assert_eq!(None, result);
            buf.extend_from_slice(&bytes[n..]);
            let result = codec.decode(&mut buf);
            assert_eq!(expected, result.unwrap().unwrap());
        }
    }

    #[test]
    fn pure_bytes_to_mspv2_checksum_error() {
        let mut codec = MSPCodec::new();
        let mut buf = BytesMut::from(vec![
            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x81,
        ]);

        let expected = io::Error::new(io::ErrorKind::InvalidInput, "wrong MSP checksum");
        let result = codec.decode(&mut buf).unwrap_err();
        assert_eq!(expected.kind(), result.kind());
        assert_eq!(format!("{}", expected), format!("{}", result));
    }

    #[test]
    fn mspv2_to_bytes() {
        let mut codec = MSPCodec::new();
        let mut buf = BytesMut::new();

        let msp = MSPMessage {
            version: MSPVersion::V2,
            direction: MSPDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: "Hello flying world".as_bytes().to_vec(),
        };
        let expected = vec![
            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
        ];
        let result = codec.encode(msp, &mut buf).unwrap();
        assert_eq!((), result);
        assert_eq!(expected, buf.to_vec());
    }
}
