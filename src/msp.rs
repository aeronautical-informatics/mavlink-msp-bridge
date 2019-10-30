use std::clone::Clone;
use std::convert::{TryFrom, TryInto};
use std::io::{self, Read, Write};

//use bytes::{Buf, BufMut, BytesMut, IntoBuf, Writer};
use byteorder::{LittleEndian, ReadBytesExt};
use crc_any::CRC;
use log::debug;

macro_rules! msp_payload {
    ( $( { $name:ident $id:expr, $($field_name:ident : $field_type:ty),+ } ),+ ) => {
        $(
            #[derive(Debug, Copy, Clone, PartialEq)]
            pub struct $name {
                $( $field_name: $field_type, )+
            }

            impl MspPayloadData for $name {
                const SIZE: usize = 0 $( + std::mem::size_of::<$field_type>() )+;
                const ID: IdType = $id;

                fn decode<R: Read>(mut r:R)->io::Result<Self>{
                    let mut buf = [0u8; Self::SIZE];
                    r.read_exact(&mut buf[..])?;
                    let mut i = 0;
                    Ok( $name {
                        $( $field_name : {
                                let size = std::mem::size_of::<$field_type>();
                                i += size;
                                <$field_type>::from_le_bytes(buf[i-size..i].try_into().unwrap())
                        }, )+
                    })
                }

                fn encode<W: Write>(&self, mut w: W)->io::Result<()>{
                    $( w.write(&self.$field_name.to_le_bytes())?; )+
                        Ok(())
                }
            }
        )+

        #[derive(Debug, Copy, Clone, PartialEq)]
        pub enum MspPayload {
            $( $name ( $name ), )+
        }

        impl MspPayloadEnum for MspPayload {
            fn size(&self)->usize {
                match self {
                    $( MspPayload::$name(_)  => $name::SIZE, )+
                }
            }

            fn decode<R: Read>(r: R, id: IdType) -> io::Result<MspPayload> {
                match id {
                    $( $id => match $name::decode(r) {
                        Ok(payload) => Ok(MspPayload::$name(payload)),
                        Err(e) => Err(e)
                    })+
                    _=> Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unknown Msp ID {}", id)))
                }
            }

            fn encode<W: Write>(&self, w: W)->io::Result<()>{
                match self {
                    $( MspPayload::$name(payload)  => payload.encode(w), )+
                }
            }
        }
    }
}

macro_rules! put {
    ( $dest: expr, [ $( $var: expr ),* ] ) => {
        $( &$dest.write( &$var.to_le_bytes())?; )*
    }
}

/// Type for MSP Id
type IdType = u16;

/// Request: Master to Slave (`<`)
/// Response: Slave to Master (`>`)
/// Error: Master to Slave or Slave to Master (`!`)
#[derive(Clone, Debug, PartialEq)]
pub enum MspDirection {
    Request,
    Response,
    Error,
}

impl From<&MspDirection> for u8 {
    fn from(d: &MspDirection) -> Self {
        match d {
            MspDirection::Request => '<' as u8,
            MspDirection::Response => '>' as u8,
            MspDirection::Error => '!' as u8,
        }
    }
}

impl TryFrom<u8> for MspDirection {
    type Error = io::Error;
    fn try_from(byte: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match byte as char {
            '<' => Ok(MspDirection::Request),
            '>' => Ok(MspDirection::Response),
            '!' => Ok(MspDirection::Error),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown Msp direction",
            )),
        }
    }
}

/// V1: (`M`)
/// V2: (`X`)
#[derive(Clone, Debug, PartialEq)]
pub enum MspVersion {
    V1,
    V2,
}

impl From<&MspVersion> for u8 {
    fn from(d: &MspVersion) -> Self {
        match d {
            MspVersion::V1 => 'M' as u8,
            MspVersion::V2 => 'X' as u8,
        }
    }
}

impl TryFrom<u8> for MspVersion {
    type Error = io::Error;
    fn try_from(byte: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
        match byte as char {
            'M' => Ok(MspVersion::V1),
            'X' => Ok(MspVersion::V2),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unknown msp version",
            )),
        }
    }
}

trait MspPayloadData {
    const ID: IdType;
    const SIZE: usize;

    fn decode<R: Read>(r: R) -> io::Result<Self>
    where
        Self: std::marker::Sized;
    fn encode<W: Write>(&self, w: W) -> io::Result<()>;
}

trait MspPayloadEnum {
    fn size(&self) -> usize;
    fn decode<R: Read>(r: R, id: IdType) -> io::Result<MspPayload>;
    fn encode<W: Write>(&self, w: W) -> io::Result<()>;
}

msp_payload![
    {MspIdent 100, version: u8, multitype: u8, msp_version: u8, capability: u32},
    {MspStatus 101, cycle_time: u16, i2c_errors_count: u16, sensor: u16,  flag: u32,  global_conf_current_set: u8}
//    {Msp_IDENT  100},
//    {Msp_STATUS  101},
//    {Msp_RAW_IMU  102},
//    {Msp_SERVO  103},
//    {Msp_MOTOR  104},
//    {Msp_SET_MOTOR  214},
//    {Msp_RC  105},
//    {Msp_SET_RAW_RC  200},
//    {Msp_RAW_GPS  106},
//    {Msp_SET_RAW_GPS  201},
//    {Msp_COMP_GPS  107},
//    {Msp_ATTITUDE  108},
//    {Msp_ALTITUDE  109},
//    {Msp_ANALOG  110},
//    {Msp_RC_TUNING  111},
//    {Msp_SET_RC_TUNING  204},
//    {Msp_PID  112},
//    {Msp_SET_PID  202},
//    {Msp_BOX  113},
//    {Msp_SET_BOX  203},
//    {Msp_MISC  114},
//    {Msp_SET_MISC  207},
//    {Msp_MOTOR_PINS  115},
//    {Msp_BOXNAMES  116},
//    {Msp_PIDNAMES  117},
//    {Msp_WP  118},
//    {Msp_SET_WP  209},
//    {Msp_BOXIDS  119},
//    {Msp_SERVO_CONF  120},
//    {Msp_SET_SERVO_CONF  212},
//    {Msp_ACC_CALIBRATION  205},
//    {Msp_MAG_CALIBRATION  206},
//    {Msp_RESET_CONF  208},
//    {Msp_SELECT_SETTING  210},
//    {Msp_SET_HEAD  211},
//    {Msp_BIND  240},
//    {Msp_EEPROM_WRITE  250},
    ];

/// A flag may only be `Some(_)` if `version == MspVersion::V2`
#[derive(Clone, Debug, PartialEq)]
pub struct MspMessage {
    pub version: MspVersion,
    pub direction: MspDirection,
    pub flag: Option<u8>,
    pub function: IdType,
    pub payload: Option<MspPayload>,
}

impl std::fmt::Display for MspMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "FC {:?}, Flag {:?} {:?}\t{})",
            self.direction,
            self.flag,
            self.function,
            format!("{:?}", self.payload)
        )
    }
}

impl MspMessage {
    pub fn checksum(&self) -> u8 {
        match self.version {
            MspVersion::V1 => {
                panic!("not yet implemented");
            }
            MspVersion::V2 => {
                let mut crc = CRC::create_crc(0xd5, 8, 0x0, 0x0, false);
                crc.digest(&[self.flag.unwrap_or(0)]);
                crc.digest(&self.function.to_le_bytes());
                match self.payload {
                    Some(payload) => {
                        let mut payload_buf: Vec<u8> = Vec::new();
                        payload.encode(&mut payload_buf).unwrap();
                        crc.digest(&payload_buf.len().to_le_bytes());
                        crc.digest(&payload_buf);
                    }
                    None => crc.digest(&0u8.to_le_bytes()),
                }
                u8::try_from(crc.get_crc()).unwrap()
            }
        }
    }

    pub fn encode<W: Write>(&self, mut w: W) -> io::Result<()> {
        match self.version {
            MspVersion::V1 => panic!("not implemented yet"),
            MspVersion::V2 => {
                put!(
                    w,
                    [
                        '$' as u8,
                        u8::from(&self.version),
                        u8::from(&self.direction),
                        self.flag.unwrap_or(0),
                        self.function
                    ]
                );

                match self.payload {
                    Some(payload) => {
                        let size: u16 = payload.size().try_into().expect("payload too big");
                        put!(w, [size]);
                        payload.encode(&mut w)?;
                    }
                    None => {
                        put!(w, [0u8]);
                    }
                }
                put!(w, [self.checksum()]);
            }
        }
        Ok(())
    }

    pub fn decode<R: Read>(mut r: R) -> io::Result<Self> {
        #[derive(Debug)]
        enum State {
            Header,
            V1Fields,
            V2Fields,
            Jumbo,
            Payload(usize),
            Checksum,
        };

        let mut state: Option<State> = None;
        let mut message = MspMessage {
            version: MspVersion::V1,
            direction: MspDirection::Error,
            flag: None,
            function: 0,
            payload: None,
        };

        loop {
            debug!("state: {:?}, message: {:?}", state, message);
            match state {
                Some(State::Header) => {
                    state = None;
                    message.version = MspVersion::try_from(r.read_u8()?)?;
                    message.direction = MspDirection::try_from(r.read_u8()?)?;
                    state = match message.version {
                        MspVersion::V1 => Some(State::V1Fields),
                        MspVersion::V2 => Some(State::V2Fields),
                    };
                }
                Some(State::V1Fields) => {
                    message.flag = None;
                    let payload_size = r.read_u8()? as usize;
                    message.function = r.read_u16::<LittleEndian>()?;
                    state = match payload_size {
                        255 => Some(State::Jumbo),
                        _ => Some(State::Payload(payload_size)),
                    };
                }
                Some(State::V2Fields) => {
                    message.flag = Some(r.read_u8()?);
                    message.function = r.read_u16::<LittleEndian>()?;
                    let payload_size = r.read_u16::<LittleEndian>()? as usize;
                    state = Some(State::Payload(payload_size));
                }
                Some(State::Jumbo) => {
                    let payload_size = r.read_u16::<LittleEndian>()? as usize;
                    state = Some(State::Payload(payload_size));
                }
                Some(State::Payload(payload_size)) if payload_size > 0 => {
                    message.payload = Some(MspPayload::decode(&mut r, message.function)?);
                    state = Some(State::Checksum);
                }
                Some(State::Payload(_)) => state = Some(State::Checksum),
                Some(State::Checksum) => {
                    state = None;
                    match message.checksum() == r.read_u8()? {
                        true => return Ok(message.clone()),
                        false => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "wrong Msp checksum",
                            ))
                        }
                    }
                }
                None => {
                    if r.read_u8()? as char == '$' {
                        state = Some(State::Header);
                    }
                }
            }
        }
    }
}

pub trait MspConnection {
    fn request(self, msg: &MspMessage) -> io::Result<MspMessage>;
}

impl<T: Read + Write> MspConnection for T
where
    T: Read + Write,
{
    fn request(mut self, msg: &MspMessage) -> io::Result<MspMessage> {
        let now = std::time::SystemTime::now();

        msg.encode(&mut self)?;
        self.flush()?;

        let response = MspMessage::decode(&mut self)?;

        debug!("time spent on Msp Request: {:?}", now.elapsed().unwrap());

        Ok(response)
    }
}

//#[cfg(test)]
//mod test {
//    use super::*;
//
//    #[test]
//    fn pure_bytes_to_mspv2() {
//        let mut codec = MspCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
//                                     0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
//        ]);
//
//        let expected = MspMessage {
//            version: MspVersion::V2,
//            direction: MspDirection::Response,
//            flag: Some(0xa5),
//            function: 0x4242,
//            payload: "Hello flying world".as_bytes().to_vec(),
//        };
//        let result = codec.decode(&mut buf);
//        assert_eq!(expected, result.unwrap().unwrap());
//    }
//
//    #[test]
//    fn pure_bytes_to_multiple_mspv2() {
//        let mut codec = MspCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x24u8, 0x58, 0x3c, 0x00, 0x64, 0x00, 0x00, 0x00, 0x8f, 0x24, 0x58, 0x3e, 0xa5, 0x42,
//                                     0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e,
//                                     0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
//        ]);
//
//        let message_1 = MspMessage {
//            version: MspVersion::V2,
//            direction: MspDirection::Request,
//            flag: Some(0),
//            function: 100,
//            payload: vec![0u8; 0],
//        };
//        let message_2 = MspMessage {
//            version: MspVersion::V2,
//            direction: MspDirection::Response,
//            flag: Some(0xa5),
//            function: 0x4242,
//            payload: "Hello flying world".as_bytes().to_vec(),
//        };
//        let result = codec.decode(&mut buf);
//        assert_eq!(message_1, result.unwrap().unwrap());
//        let result = codec.decode(&mut buf);
//        assert_eq!(message_2, result.unwrap().unwrap());
//    }
//
//    #[test]
//    fn noised_bytes_to_mspv2() {
//        let mut codec = MspCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x30, 0x60, 0x13, 0x24, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c,
//                                     0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c,
//                                     0x64, 0x82, 0x25,
//        ]);
//
//        let expected = MspMessage {
//            version: MspVersion::V2,
//            direction: MspDirection::Response,
//            flag: Some(0xa5),
//            function: 0x4242,
//            payload: "Hello flying world".as_bytes().to_vec(),
//        };
//        let result = codec.decode(&mut buf);
//        assert_eq!(expected, result.unwrap().unwrap());
//    }
//
//    #[test]
//    fn pure_bytes_to_mspv2_partial() {
//        let mut codec = MspCodec::new();
//        let expected = MspMessage {
//            version: MspVersion::V2,
//            direction: MspDirection::Response,
//            flag: Some(0xa5),
//            function: 0x4242,
//            payload: "Hello flying world".as_bytes().to_vec(),
//        };
//
//        let bytes = vec![
//            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
//            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
//        ];
//
//        for n in 0..bytes.len() {
//            let mut buf = BytesMut::with_capacity(bytes.len());
//            buf.extend_From_slice(&bytes[..n]);
//            let result = codec.decode(&mut buf).unwrap();
//            assert_eq!(None, result);
//            buf.extend_From_slice(&bytes[n..]);
//            let result = codec.decode(&mut buf);
//            assert_eq!(expected, result.unwrap().unwrap());
//        }
//    }
//
//    #[test]
//    fn pure_bytes_to_mspv2_checksum_error() {
//        let mut codec = MspCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
//                                     0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x81,
//        ]);
//
//        let expected = io::Error::new(io::ErrorKind::InvalidInput, "wrong Msp checksum");
//        let result = codec.decode(&mut buf).unwrap_err();
//        assert_eq!(expected.kind(), result.kind());
//        assert_eq!(format!("{}", expected), format!("{}", result));
//    }
//
//    #[test]
//    fn mspv2_to_bytes() {
//        let mut codec = MspCodec::new();
//        let mut buf = BytesMut::new();
//
//        let msp = MspMessage {
//            version: MspVersion::V2,
//            direction: MspDirection::Response,
//            flag: Some(0xa5),
//            function: 0x4242,
//            payload: "Hello flying world".as_bytes().to_vec(),
//        };
//        let expected = vec![
//            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
//            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
//        ];
//        let result = codec.encode(msp, &mut buf).unwrap();
//        assert_eq!((), result);
//        assert_eq!(expected, buf.to_vec());
//    }
//}
