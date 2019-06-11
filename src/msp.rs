//use bytes::{Bytes, BytesMut, Buf, BufMut};
use crc_any::CRC;
use std::convert::TryFrom;
use std::num::ParseIntError;

// Request: Slave < Master
// Response: Slave > Master
// Error: Slave|Master > Master|Slave

#[derive(Debug, PartialEq)]
pub enum Direction {
    Request,
    Response,
    Error,
}

#[derive(Debug, PartialEq)]
pub struct Frame {
    pub direction: Direction,
    pub flag: Option<u8>,
    pub function: u16,
    pub payload: Vec<u8>,
}

impl std::convert::TryFrom<u8> for Direction {
    type Error = &'static str;
    fn try_from(byte: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match byte as char {
            '<' => Ok(Direction::Request),
            '>' => Ok(Direction::Response),
            '!' => Ok(Direction::Error),
            _ => Err("unknown message direction"),
        }
    }
}

impl From<&Direction> for u8 {
    fn from(d: &Direction) -> Self {
        match d {
            Direction::Request => '<' as u8,
            Direction::Response => '>' as u8,
            Direction::Error => '!' as u8,
        }
    }
}

impl std::fmt::Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "({:?} {:?} {:?}\t{:?})",
            self.direction, self.flag, self.function, self.payload
        )
    }
}

impl Frame {
    pub fn build_v1(&self) -> Vec<u8> {
        let payload_length = u16::try_from(self.payload.len()).expect("payload too long");
        let total_length = 9 + self.payload.len();

        //poly: $t, width: usize, init: $t, xorout: $t, reflect: bool<Paste>
        let mut crc = CRC::create_crc(0xd5, 8, 0x0, 0x0, false);

        let mut result = vec![0 as u8; total_length];
        result[0] = '$' as u8;
        result[1] = 'X' as u8;
        result[2] = u8::from(&self.direction);
        result[3] = self.flag.unwrap_or(0);
        result.splice(4..5, self.function.to_le_bytes().iter().cloned());
        result.splice(6..7, payload_length.to_le_bytes().iter().cloned());
        result.splice(8.., self.payload.iter().cloned());
        crc.digest(&result[3..]);
        let crc = u8::try_from(crc.get_crc()).unwrap();

        result.push(crc);

        result
    }

    pub fn build_v2(&self) -> Vec<u8> {
        let payload_length = u16::try_from(self.payload.len()).expect("payload too long");
        let total_length = 9 + self.payload.len();

        //poly: $t, width: usize, init: $t, xorout: $t, reflect: bool<Paste>
        let mut crc = CRC::create_crc(0xd5, 8, 0x0, 0x0, false);

        let mut result = vec![0 as u8; total_length];
        result[0] = '$' as u8;
        result[1] = 'X' as u8;
        result[2] = u8::from(&self.direction);
        result[3] = self.flag.unwrap_or(0);
        result.splice(4..5, self.function.to_le_bytes().iter().cloned());
        result.splice(6..7, payload_length.to_le_bytes().iter().cloned());
        result.splice(8.., self.payload.iter().cloned());
        crc.digest(&result[3..]);
        let crc = u8::try_from(crc.get_crc()).unwrap();

        result.push(crc);

        result
    }
}

impl TryFrom<&[u8]> for Frame {
    type Error = &'static str;

    fn try_from(frame: &[u8]) -> Result<Self, Self::Error> {
        if frame.len() < 2 {
            return Err("frame to short to be valid");
        }

        // missing: check crc

        if frame.starts_with(&['$' as u8, 'X' as u8]) {
            let mut crc = CRC::create_crc(0xd5, 8, 0x0, 0x0, false);

            let direction = Direction::try_from(frame[2])?;
            let flag = Some(frame[3]);
            let function = u16::from_le_bytes([frame[4], frame[5]]);
            let payload_length = u16::from_le_bytes([frame[6], frame[7]]);
            let payload = frame[8..8 + payload_length as usize].to_vec();
            crc.digest(&frame[3..8 + payload_length as usize]);
            let crc = u8::try_from(crc.get_crc()).unwrap();
            if &crc != frame.last().unwrap() {
                return Err("wrong crc");
            }

            return Ok(Frame {
                direction: direction,
                flag: flag,
                function: function,
                payload: payload,
            });
        } else if frame.starts_with(&['$' as u8, 'M' as u8]) {

        } else {
            return Err("frame has invalid start sequence");
        }

        Err("not implemented yet")
    }

    //fn parse_v1

    //fn parse_v2
}

#[test]
fn msp_to_bytes_1_test() {
    let sample = vec![0x24u8, 0x58, 0x3c, 0x00, 0x64, 0x00, 0x00, 0x00, 0x8f];
    let message = Frame {
        direction: Direction::Request,
        flag: Some(0),
        function: 100,
        payload: vec![0u8; 0],
    };
    let bytes = message.build_v2();
    assert_eq!(sample, bytes)
}

#[test]
fn msp_to_bytes_2_test() {
    let sample = vec![
        0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66,
        0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
    ];
    let message = Frame {
        direction: Direction::Response,
        flag: Some(0xa5),
        function: 0x4242,
        payload: "Hello flying world".as_bytes().to_vec(),
    };
    let bytes = message.build_v2();
    assert_eq!(sample, bytes)
}

#[test]
fn bytes_to_msp_1_test() {
    let sample = [
        0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66,
        0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
    ];

    let msp_1 = Frame::try_from(&sample[..]).unwrap();
    let msp_2 = Frame {
        direction: Direction::Response,
        flag: Some(0xa5),
        function: 0x4242,
        payload: "Hello flying world".as_bytes().to_vec(),
    };
    assert_eq!(msp_1, msp_2)
}
