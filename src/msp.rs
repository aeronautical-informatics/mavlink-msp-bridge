use std::clone::Clone;
use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Display};
use std::io::{self, Read, Write};
use std::mem::size_of;

use crc_any::CRC;

macro_rules! msp_codec {
    ( $name:ident $id:expr ) => {
        pub struct $name {}
        impl MspPayload for $name {
            const SIZE: usize = 0 ;
            const ID: IdType = $id;

            fn decode<R: Read>(r:&mut R)->io::Result<Self>{
                Ok($name{})
            }

            fn encode<W: Write>(&self, w: &mut W)->io::Result<()>{
                Ok()
            }
        }
    };

    ( $name:ident $id:expr, $($field_name:ident : $field_type:ty),+ ) => {
        #[derive(Debug, Copy, Clone, PartialEq)]
        pub struct $name {
            $( pub $field_name: $field_type, )+
        }

        impl MspPayload for $name {
            const SIZE: usize = 0 $( + size_of::<$field_type>() )+;
            const ID: IdType = $id;

            fn decode<R: Read>(r:&mut R)->io::Result<Self>{
                let mut buf = [0u8; Self::SIZE];
                r.read_exact(&mut buf[..])?;
                let mut i = 0;
                Ok( $name {
                    $( $field_name : {
                        let size = size_of::<$field_type>();
                        i += size;
                        <$field_type>::from_le_bytes(buf[i-size..i].try_into().unwrap())
                    }, )+
                })
            }


            fn encode<W: Write>(&self, w: &mut W)->io::Result<()>{
                let mut buf = [0u8; Self::SIZE];
                let mut i = 0;
                $(
                    let size = size_of::<$field_type>();
                    i+=size;
                    buf[i-size..i].copy_from_slice(&self.$field_name.to_le_bytes()[..]);
                )+
                    w.write_all(&buf[..])
            }
        }
    };

    ($name:ident $id:expr, [$type:ty; $size:expr]) => {
        #[derive(Clone, Debug, PartialEq)]
        struct $name( [$type; $size] );

        impl MspPayload for $name {
            const SIZE: usize = $size * size_of::<$type>();
            const ID: IdType = $id;

            fn decode<R: Read>(r: &mut R) -> io::Result<$name> {
                let mut buf = [0u8; Self::SIZE];
                r.read_exact(&mut buf[..])?;

                let mut payload = [0 as $type; $size];
                let mut i = 0;
                for e in &mut payload {
                    let size = size_of::<$type>();
                    i += size;
                    *e = <$type>::from_le_bytes(buf[i-size..i].try_into().unwrap());
                }
                Ok($name(payload))
            }

            fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
                let mut buf = [0u8; Self::SIZE];
                let mut i = 0;

                for e in &self.0 {
                    let size = size_of::<$type>();
                    i+= size;
                    buf[i-size..i].copy_from_slice(&e.to_le_bytes()[..]);
                }
                w.write_all(&buf[..])
            }
        }
    };
}

#[cfg(test)]
macro_rules! msp_test {
    ( $name:ident $id:expr ) => {
        mod $name {
            use super::super::*;

            #[test]
            fn message_encode_decode_test(){
                let mut buf: Vec<u8> = Vec::new();
                let message = MspMessage {
                    version: MspVersion::V2,
                    direction: MspDirection::Response,
                    flag: Some(random()),
                    function: $id,
                    payload: Some( $name{}),
                };
                message.encode(&mut buf).expect("unable to encode");
                let new_message = MspMessage::decode(&mut &buf[..]).expect("unable to decode");
                let mut new_buf: Vec<u8> = Vec::new();
                new_message.encode(&mut new_buf).expect("unable to encode");
                assert_eq!(message, new_message);
                assert_eq!(buf, new_buf);
            }
        }
    };

    ( $name:ident $id:expr, $($field_name:ident : $field_type:ty),+ ) => {
        #[allow(non_snake_case)]
        #[cfg(test)]
        mod $name {
            use rand::random;

            use super::super::*;

            #[test]
            fn encode_decode_test(){
                let mut buf: Vec<u8> = Vec::new();
                let payload = $name { $( $field_name : random(), )+ };
                payload.encode(&mut buf).expect("unable to encode");
                let new_payload = $name::decode(&mut &buf[..]).expect("unable to decode");
                let mut new_buf: Vec<u8> = Vec::new();
                new_payload.encode(&mut new_buf).expect("unable to encode");
                assert_eq!(payload, new_payload);
                assert_eq!(buf, new_buf);
            }

            #[test]
            fn message_encode_decode_test(){
                let mut buf: Vec<u8> = Vec::new();
                let message = MspMessage {
                    version: MspVersion::V2,
                    direction: MspDirection::Response,
                    flag: Some(random()),
                    function: $id,
                    payload: Some( $name { $( $field_name : random(), )+ }),
                };
                message.encode(&mut buf).expect("unable to encode");
                let new_message = MspMessage::decode(&mut &buf[..]).expect("unable to decode");
                let mut new_buf: Vec<u8> = Vec::new();
                new_message.encode(&mut new_buf).expect("unable to encode");
                assert_eq!(message, new_message);
                assert_eq!(buf, new_buf);
            }
        }
    };
    ( $name:ident $id:expr, [$type:ty; $size:expr] ) => {
        #[allow(non_snake_case)]
        mod $name {
            use rand::random;

            use super::super::*;

            #[test]
            fn encode_decode_test(){
                let mut buf: Vec<u8> = Vec::new();
                let mut payload = $name([0 as $type; $size]);
                for ref mut e in &mut payload.0 {
                    *e = &mut random();
                }
                payload.encode(&mut buf).expect("unable to encode");
                let new_payload = $name::decode(&mut &buf[..]).expect("unable to decode");
                let mut new_buf: Vec<u8> = Vec::new();
                new_payload.encode(&mut new_buf).expect("unable to encode");
                assert_eq!(payload, new_payload);
                assert_eq!(buf, new_buf);
            }

            #[test]
            fn message_encode_decode_test(){
                let mut buf: Vec<u8> = Vec::new();
                let mut payload = $name([0 as $type; $size]);
                for  ref mut e in &mut payload.0 {
                    *e = &mut random();
                }
                let message = MspMessage {
                    version: MspVersion::V2,
                    direction: MspDirection::Response,
                    flag: Some(random()),
                    function: $id,
                    payload: Some( payload ),
                };
                message.encode(&mut buf).expect("unable to encode");
                let new_message = MspMessage::decode(&mut &buf[..]).expect("unable to decode");
                let mut new_buf: Vec<u8> = Vec::new();
                new_message.encode(&mut new_buf).expect("unable to encode");
                assert_eq!(message, new_message);
                assert_eq!(buf, new_buf);
            }
        }
    };
}

macro_rules! msp_payload {
    ( $( { $name:ident $id:expr, $($fields:tt)* } ),* ) => {
        $(
            msp_codec!{$name $id, $($fields)* }
        )*

        #[cfg(test)]
        mod test_generated {
            $(
                msp_test!{$name $id, $($fields)*}
            )*
        }
    };
}

macro_rules! get {
    ( $src: expr, $type:ty ) => {{
        let mut buf = [0u8; size_of::<$type>()];
        $src.read_exact(&mut buf[..])?;
        <$type>::from_le_bytes(buf[..].try_into().unwrap())
    }};
}

/// Type for MSP Id
type IdType = u16;

/// Type for MSP payload len
type LenType = u16;

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

pub trait MspPayload {
    const ID: IdType;
    const SIZE: usize;

    fn decode<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: std::marker::Sized;
    fn encode<W: Write>(&self, w: &mut W) -> io::Result<()>;
}

//msp_payload!{MspIdent 100, version: u8, multitype: u8, msp_version: u8, capability: u32}
msp_payload! {
    { MspIdent 100, version: u8, multitype: u8, msp_version: u8, capability: u32},
    { MspStatus 101, cycle_time: u16, i2c_errors_count: u16, sensor: u16,  flag: u32,  global_conf_current_set: u8 },
    { MspRawImu 102, accx: i16, accy: i16, accz: i16, gyrx: i16, gyry: i16, gyrz: i16, magx: i16, magy: i16, magz: i16 },
    { MspServo 103, [u16;16]},
    { MspMotor 104, [u16; 16]},
    { MspSetMotor 214, [u16; 16]},
    { MspRc 105, [u16; 16]},
    { MspSetRawRc  200, [u16; 16]},
    { MspRawGps  106, fix:u8, num_sat:u8, coord_lat: i32, coord_lon: i32, altitude: u16, speed:u16, ground_course: u16 },
    { MspSetRawGps  201, fix:u8, num_sat:u8, coord_lat: i32, coord_lon: i32, altitude: u16, speed:u16},
    { MspCompGps 107, distance_to_home: u16, direction_to_home: i16, update: u8},
    { MspAttitude 108, angx: i16, angy: i16, heading: i16},
    { MspAltitude 109, estimated_alt: i32, vario: i16},
    { MspAnalog 110, vbat: u8, int_power_meter_sum: u16, rssi: u16, amperage: u16},
    { MspRcTuning 111, rc_rate:u8, rc_expo: u8, roll_pitch_rate: u8, yaw_rate: u8, dyn_thr_pid:u8, throttle_mid: u8, throttle_expo: u8},
    { MspSetRcTuning 204, rc_rate:u8, rc_expo: u8, roll_pitch_rate: u8, yaw_rate: u8, dyn_thr_pid:u8, throttle_mid: u8, throttle_expo: u8},
    //{Msp_PID  112},
    //{Msp_SET_PID  202},
    //{Msp_BOX  113},
    //{Msp_SET_BOX  203},
    //{Msp_MISC  114},
    //{Msp_SET_MISC  207},
    { MspMotorPins 115, [u8;8]},
    //{Msp_BOXNAMES  116},
    //{Msp_PIDNAMES  117},
    { MspWp 118, wp_no: u8, lat:i32, lon: i32, alt_hold: u32, heading: i16, time_to_stay:u16, nav_flag: u8},
    { MspSetWp 209, wp_no: u8, lat:i32, lon: i32, alt_hold: u32, heading: i16, time_to_stay:u16, nav_flag: u8},
    //{Msp_SET_WP  209},
    //{Msp_BOXIDS  119},
    //{Msp_SERVO_CONF  120},
    //{Msp_SET_SERVO_CONF  212},
    //{Msp_ACC_CALIBRATION  205},
    //{Msp_MAG_CALIBRATION  206},
    //{Msp_RESET_CONF  208},
    //{Msp_SELECT_SETTING  210},
    { MspSetHead 211, mag_hold: i16}
    //{Msp_BIND  240},
    //{ Msp_EEPROM_WRITE 250}
}

/// A flag may only be `Some(_)` if `version == MspVersion::V2`
#[derive(Clone, Debug, PartialEq)]
pub struct MspMessage<P: MspPayload> {
    pub version: MspVersion,
    pub direction: MspDirection,
    pub flag: Option<u8>,
    pub function: IdType,
    pub payload: Option<P>,
}

impl<P> Display for MspMessage<P>
where
    P: MspPayload + Clone + Debug,
{
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

impl<P> MspMessage<P>
where
    P: MspPayload + Clone + Debug,
{
    /// serializes message omiting the checksum
    fn ser(&self) -> io::Result<Vec<u8>> {
        match self.version {
            MspVersion::V1 => {
                const LEN_OFFSET: usize = 4 + size_of::<IdType>();
                let mut buf =
                    vec![0u8; 4 * size_of::<u8>() + size_of::<IdType>() + size_of::<LenType>()];
                Ok(buf)
            }
            MspVersion::V2 => {
                const LEN_OFFSET: usize = 4 + size_of::<IdType>();

                let mut buf =
                    vec![0u8; 4 * size_of::<u8>() + size_of::<IdType>() + size_of::<LenType>()];
                buf[0] = '$' as u8;
                buf[1] = u8::from(&self.version);
                buf[2] = u8::from(&self.direction);
                buf[3] = self.flag.unwrap_or(0);
                buf[4..LEN_OFFSET].copy_from_slice(&self.function.to_le_bytes()[..]);
                let len: u16 = match self.payload {
                    Some(_) => P::SIZE.try_into().expect("payload too big"),
                    _ => 0,
                };
                buf[LEN_OFFSET..].copy_from_slice(&len.to_le_bytes()[..]);

                if let Some(payload) = &self.payload {
                    let mut payload_buf = vec![0u8; P::SIZE];
                    payload.encode(&mut &mut payload_buf[..])?;
                    buf.append(&mut payload_buf);
                }
                Ok(buf)
            }
        }
    }

    /// calculates the checksum for the given message
    pub fn checksum(&self) -> u8 {
        match self.version {
            MspVersion::V1 => {
                let mut xor = 0;
                let buf = &self.ser().unwrap();
                for byte in &buf[2..] {
                    xor ^= byte;
                }
                xor
            }
            MspVersion::V2 => {
                let mut crc = CRC::create_crc(0xd5, 8, 0x0, 0x0, false);
                let buf = &self.ser().unwrap();
                crc.digest(&buf[3..]);
                crc.get_crc().try_into().unwrap()
            }
        }
    }

    /// encodes the message to something which can be written to
    pub fn encode<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let buf = &self.ser()?;
        w.write_all(&buf)?;
        w.write_all(&self.checksum().to_le_bytes())
    }

    /// decodes a message from something which can be read from
    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self> {
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
            match state {
                Some(State::Header) => {
                    message.version = MspVersion::try_from(get!(r, u8))?;
                    message.direction = MspDirection::try_from(get!(r, u8))?;
                    state = Some(match message.version {
                        MspVersion::V1 => State::V1Fields,
                        MspVersion::V2 => State::V2Fields,
                    });
                }
                Some(State::V1Fields) => {
                    message.flag = None;
                    let payload_size = get!(r, u8) as usize;
                    message.function = get!(r, u16);
                    state = Some(match payload_size {
                        255 => State::Jumbo,
                        _ => State::Payload(payload_size),
                    });
                }
                Some(State::V2Fields) => {
                    message.flag = Some(get!(r, u8));
                    message.function = get!(r, u16);
                    let payload_size = get!(r, u16) as usize;
                    state = Some(match payload_size {
                        0 => State::Checksum,
                        _ => State::Payload(payload_size),
                    });
                }
                Some(State::Jumbo) => {
                    let payload_size = get!(r, u16) as usize;
                    state = Some(State::Payload(payload_size));
                }
                Some(State::Payload(payload_size)) if payload_size > 0 => {
                    message.payload = Some(P::decode(r)?);
                    state = Some(State::Checksum);
                }
                Some(State::Payload(_)) => state = Some(State::Checksum),
                Some(State::Checksum) => match message.checksum() == get!(r, u8) {
                    true => return Ok(message.clone()),
                    false => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "wrong Msp checksum",
                        ))
                    }
                },
                None => {
                    if get!(r, u8) as char == '$' {
                        state = Some(State::Header);
                    }
                }
            }
        }
    }

    /// tries to fetch a payload from a ressource that both allows us to read and write from/to it
    pub fn fetch<T: Read + Write>(conn: &mut T) -> io::Result<P> {
        let msg: Self = MspMessage {
            version: MspVersion::V2,
            direction: MspDirection::Request,
            flag: None,
            function: P::ID,
            payload: None,
        };
        match msg.request(conn)?.payload {
            Some(p) => Ok(p),
            None => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "received empty MSP payload",
            )),
        }
    }

    /// sends the message to
    pub fn request<T: Read + Write>(&self, conn: &mut T) -> io::Result<Self> {
        let t_start = std::time::Instant::now();
        self.encode(conn)?;
        let t_encode = t_start.elapsed();
        let response = Self::decode(conn)?;
        let t_total = t_start.elapsed();
        if cfg!(time_metrics) {
            println!(
                "time spent: total {:?} encode {:?}, decode {:?}",
                t_total,
                t_encode,
                t_total - t_encode
            );
        }
        Ok(response)
    }
}

pub trait MspConnection: Read + Write {}
impl<T: Read + Write> MspConnection for T {}

#[cfg(test)]
mod test_handwritten {
    use super::*;

    #[test]
    fn pure_bytes_to_mspv2() {
        let buf = [0x24u8, 0x58, 0x3c, 0, 0x64, 0, 0, 0, 0x8f];

        let message: MspMessage<MspIdent> = MspMessage {
            version: MspVersion::V2,
            direction: MspDirection::Request,
            flag: Some(0),
            function: 100,
            payload: None,
        };

        let new_message = MspMessage::decode(&mut &buf[..]).expect("unable to decode new_message");
        let mut new_buf = [0u8; 9];

        message
            .encode(&mut &mut new_buf[..])
            .expect("unable to encode message");

        assert_eq!(buf, new_buf);
        assert_eq!(message, new_message);
    }

    #[test]
    fn pure_bytes_to_mspv2_payload() {
        msp_codec! {Special 0x4242, [u8;18]}

        let buf = vec![
            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x82,
        ];

        let message: MspMessage<Special> = MspMessage {
            version: MspVersion::V2,
            direction: MspDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: Some(Special(
                "Hello flying world"
                    .as_bytes()
                    .try_into()
                    .expect("unable to convert to array"),
            )),
        };

        let new_message: MspMessage<Special> =
            MspMessage::decode(&mut &buf[..]).expect("unable to decode new_message");

        let mut new_buf = vec![0u8; 27];
        message
            .encode(&mut &mut new_buf[..])
            .expect("unable to encode message");

        assert_eq!(buf, new_buf);
        assert_eq!(message, new_message);
    }

    #[test]
    fn noised_bytes_to_mspv2() {
        msp_codec! {Special 0x4242, [u8;18]}

        let buf = vec![
            0x30, 0x60, 0x13, 0x24, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c,
            0x6c, 0x6f, 0x20, 0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c,
            0x64, 0x82, 0x25,
        ];

        let message: MspMessage<Special> = MspMessage {
            version: MspVersion::V2,
            direction: MspDirection::Response,
            flag: Some(0xa5),
            function: 0x4242,
            payload: Some(Special(
                "Hello flying world"
                    .as_bytes()
                    .try_into()
                    .expect("unable to convert to array"),
            )),
        };

        let new_message: MspMessage<Special> =
            MspMessage::decode(&mut &buf[..]).expect("unable to decode new_message");

        assert_eq!(message, new_message);
    }

    #[test]
    fn pure_bytes_to_mspv2_checksum_error() {
        msp_codec! {Special 0x4242, [u8;18]}

        let buf = vec![
            0x24u8, 0x58, 0x3e, 0xa5, 0x42, 0x42, 0x12, 0x00, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20,
            0x66, 0x6c, 0x79, 0x69, 0x6e, 0x67, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x81,
        ];

        let result: io::Result<MspMessage<Special>> = MspMessage::decode(&mut &buf[..]);
        assert!(result.is_err());
    }

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
}
