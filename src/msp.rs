use std::cell::RefCell;
use std::clone::Clone;
use std::convert::TryFrom;
use std::io::{self, Read, Write};

//use bytes::{Buf, BufMut, BytesMut, IntoBuf, Writer};
use byteorder::{BigEndian, LittleEndian, ReadBytesExt, WriteBytesExt};
use crc_any::CRC;
use log::{debug};
use mavlink::common;

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
            MSPVersion::V1 => 'm' as u8,
            MSPVersion::V2 => 'x' as u8,
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
                "unknown msp version",
            )),
        }
    }
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum MSPCommand {
    MSP_IDENT = 100,
    MSP_STATUS = 101,
    MSP_RAW_IMU = 102,
    MSP_SERVO = 103,
    MSP_MOTOR = 104,
    MSP_SET_MOTOR = 214,
    MSP_RC = 105,
    MSP_SET_RAW_RC = 200,
    MSP_RAW_GPS = 106,
    MSP_SET_RAW_GPS = 201,
    MSP_COMP_GPS = 107,
    MSP_ATTITUDE = 108,
    MSP_ALTITUDE = 109,
    MSP_ANALOG = 110,
    MSP_RC_TUNING = 111,
    MSP_SET_RC_TUNING = 204,
    MSP_PID = 112,
    MSP_SET_PID = 202,
    MSP_BOX = 113,
    MSP_SET_BOX = 203,
    MSP_MISC = 114,
    MSP_SET_MISC = 207,
    MSP_MOTOR_PINS = 115,
    MSP_BOXNAMES = 116,
    MSP_PIDNAMES = 117,
    MSP_WP = 118,
    MSP_SET_WP = 209,
    MSP_BOXIDS = 119,
    MSP_SERVO_CONF = 120,
    MSP_SET_SERVO_CONF = 212,
    MSP_ACC_CALIBRATION = 205,
    MSP_MAG_CALIBRATION = 206,
    MSP_RESET_CONF = 208,
    MSP_SELECT_SETTING = 210,
    MSP_SET_HEAD = 211,
    MSP_BIND = 240,
    MSP_EEPROM_WRITE = 250,
}

//impl From<&MSPCommand> for u16 {
//    fn from(d: &MSPCommand) -> Self {
//        match d {
//            MSPVersion::v1 => 'm' as u8,
//            MSPVersion::v2 => 'x' as u8,
//        }
//    }
//}
//
//impl TryFrom<u8> for MSPCommand{
//    type Error = io::Error;
//    fn try_from(byte: u8) -> Result<Self, <Self as TryFrom<u8>>::Error> {
//        match byte as char {
//            'm' => Ok(MSPVersion::v1),
//            'x' => Ok(MSPVersion::v2),
//            _ => Err(io::Error::new(
//                io::ErrorKind::invalidinput,
//                "unknown msp version",
//            )),
//        }
//    }
//}

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
            "FC {:?}, Flag {:?} {:?}\t{})",
            self.direction,
            self.flag,
            self.function,
            self.payload_to_string()
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

    pub fn to_vec(&self) -> Vec<u8> {
        match self.version {
            MSPVersion::V1 => panic!("not implemented yet"),
            MSPVersion::V2 => {
                let mut result = Vec::with_capacity(9 + self.payload.len());
                result.extend(b"$X");
                result.push(u8::from(&self.direction));
                result.push(self.flag.unwrap_or(0));
                result.extend(&self.function.to_le_bytes()[..]);
                let payload_size = u16::try_from(self.payload.len()).expect("payload too big");
                result.extend(&payload_size.to_le_bytes()[..]);
                result.extend(&self.payload);
                result.push(self.checksum());
                result
            }
        }
    }

    pub fn payload_to_string(&self) -> String {
        std::str::from_utf8(&self.payload)
            .unwrap_or(&format!("{:?}", self.payload))
            .to_string()
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
        let mut message = MSPMessage {
            version: MSPVersion::V1,
            direction: MSPDirection::Error,
            flag: None,
            function: 0,
            payload: Vec::new(),
        };

        loop {
            debug!("state: {:?}, message: {:?}", state, message);
            match state {
                Some(State::Header) => {
                    state = None;
                    message.version = MSPVersion::try_from(r.read_u8()?)?;
                    message.direction = MSPDirection::try_from(r.read_u8()?)?;
                    state = match message.version {
                        MSPVersion::V1 => Some(State::V1Fields),
                        MSPVersion::V2 => Some(State::V2Fields),
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
                Some(State::Payload(payload_size)) => {
                    message.payload.resize(payload_size, 0);
                    r.read_exact(&mut message.payload)?;
                    state = Some(State::Checksum);
                }
                Some(State::Checksum) => {
                    state = None;
                    match message.checksum() == r.read_u8()? {
                        true => return Ok(message.clone()),
                        false => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "wrong MSP checksum",
                            ))
                        }
                    }
                }
                None => {
                    if r.read_u8()? as char == '$' {
                        state = Some(State::Header);
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct MSPConnection<T: Read + Write> {
    socket_cell: RefCell<T>,
}

impl<T: Read + Write> MSPConnection<T> {
    pub fn new(socket: T) -> Self {
        debug!("setting up new MSPConnection");
        MSPConnection {
            socket_cell: RefCell::new(socket),
        }
    }

    pub fn request(&mut self, msg: &MSPMessage) -> Result<MSPMessage, io::Error> {
        let now = std::time::SystemTime::now();
        let socket = self.socket_cell.get_mut();

        let buf = msg.to_vec();
        //msg.encode(&socket);
        socket.write(&buf)?;
        socket.flush()?;

        let response = MSPMessage::decode(socket)?;

        debug!("time spent on MSP Request: {:?}", now.elapsed().unwrap());

        Ok(response)
    }

    pub fn generate_mav_message(&mut self, _id: u32) -> Option<common::MavMessage> {
        Some(common::MavMessage::HEARTBEAT(common::HEARTBEAT_DATA {
            custom_mode: 0,
            mavtype: mavlink::common::MavType::MAV_TYPE_QUADROTOR,
            autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
            base_mode: mavlink::common::MavModeFlag::MAV_MODE_FLAG_SAFETY_ARMED,
            system_status: mavlink::common::MavState::MAV_STATE_ACTIVE,
            mavlink_version: 0x3,
        }))
    }
}

//#[cfg(test)]
//mod test {
//    use super::*;
//
//    #[test]
//    fn pure_bytes_to_mspv2() {
//        let mut codec = MSPCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
//                                     0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
//        ]);
//
//        let expected = MSPMessage {
//            version: MSPVersion::V2,
//            direction: MSPDirection::Response,
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
//        let mut codec = MSPCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x24u8, 0x58, 0x3c, 0x00, 0x64, 0x00, 0x00, 0x00, 0x8f, 0x24, 0x58, 0x3e, 0xa5, 0x42,
//                                     0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e,
//                                     0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
//        ]);
//
//        let message_1 = MSPMessage {
//            version: MSPVersion::V2,
//            direction: MSPDirection::Request,
//            flag: Some(0),
//            function: 100,
//            payload: vec![0u8; 0],
//        };
//        let message_2 = MSPMessage {
//            version: MSPVersion::V2,
//            direction: MSPDirection::Response,
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
//        let mut codec = MSPCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x30, 0x60, 0x13, 0x24, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c,
//                                     0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c,
//                                     0x64, 0x82, 0x25,
//        ]);
//
//        let expected = MSPMessage {
//            version: MSPVersion::V2,
//            direction: MSPDirection::Response,
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
//        let mut codec = MSPCodec::new();
//        let expected = MSPMessage {
//            version: MSPVersion::V2,
//            direction: MSPDirection::Response,
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
//        let mut codec = MSPCodec::new();
//        let mut buf = BytesMut::From(vec![
//                                     0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
//                                     0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x81,
//        ]);
//
//        let expected = io::Error::new(io::ErrorKind::InvalidInput, "wrong MSP checksum");
//        let result = codec.decode(&mut buf).unwrap_err();
//        assert_eq!(expected.kind(), result.kind());
//        assert_eq!(format!("{}", expected), format!("{}", result));
//    }
//
//    #[test]
//    fn mspv2_to_bytes() {
//        let mut codec = MSPCodec::new();
//        let mut buf = BytesMut::new();
//
//        let msp = MSPMessage {
//            version: MSPVersion::V2,
//            direction: MSPDirection::Response,
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
