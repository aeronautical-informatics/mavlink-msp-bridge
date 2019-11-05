use std::io::{self, Read, Write};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use log::{debug, error, trace};
use mavlink::common::MavMessage::*;
use mavlink::common::*;
use mavlink::{MavConnection, MavHeader};

use crate::msp::*;
use crate::Config;

type MavResponse = io::Result<(MavHeader, MavMessage)>;

pub struct WrappedMAVConnection {
    system_id: u8,
    rx: mpsc::Receiver<MavResponse>,
    conn: Arc<dyn MavConnection + Send + Sync>,
}

impl WrappedMAVConnection {
    pub fn new(conf: &Config) -> Self {
        let mavconn: Arc<dyn MavConnection + Send + Sync> =
            mavlink::connect(&conf.mavlink_listen).unwrap().into();

        let (tx, rx) = mpsc::channel();
        thread::spawn({
            let mavconn = mavconn.clone();
            move || loop {
                tx.send(mavconn.recv())
                    .expect("broken MPSC in WrappedMAVConnection");
            }
        });

        WrappedMAVConnection {
            system_id: conf.mavlink_system_id,
            rx: rx,
            conn: mavconn,
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> MavResponse {
        match self.rx.recv_timeout(timeout) {
            Ok(mav_response) => {
                debug!("received: {:?}", mav_response);
                mav_response
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "timed out waiting for MAVMessage",
            )),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                error!("MAVLink thread died, cannot continue");
                panic!();
            }
        }
    }

    pub fn send(&self, data: &MavMessage) -> io::Result<()> {
        let mut header = mavlink::MavHeader::get_default_header();
        header.system_id = self.system_id;
        debug!("sending: {:?}", data);
        self.conn.send(&header, data)
    }
}

pub fn generate<T: Read + Write>(mut mspconn: &mut T, id: u32) -> io::Result<MavMessage> {
    match id {
        0 => Ok(HEARTBEAT(HEARTBEAT_DATA {
            custom_mode: 0,
            mavtype: MavType::MAV_TYPE_GENERIC,
            autopilot: MavAutopilot::MAV_AUTOPILOT_GENERIC_WAYPOINTS_AND_SIMPLE_NAVIGATION_ONLY,
            base_mode: MavModeFlag::empty(),
            system_status: MavState::MAV_STATE_STANDBY,
            mavlink_version: 0x3,
        })),
        22 => Ok(PARAM_VALUE(PARAM_VALUE_DATA {
            param_value: 0.,
            param_count: 0,
            param_index: 0,
            param_id: [
                ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
            ],
            param_type: MavParamType::MAV_PARAM_TYPE_UINT8,
        })),
        27 => {
            let payload: MspRawImu = MspMessage::fetch(&mut mspconn)?;
            Ok(RAW_IMU(RAW_IMU_DATA {
                time_usec: 0,
                xacc: payload.accx,
                yacc: payload.accy,
                zacc: payload.accz,
                xgyro: payload.gyrx,
                ygyro: payload.gyry,
                zgyro: payload.gyrz,
                xmag: payload.magx,
                ymag: payload.magy,
                zmag: payload.magz,
            }))
        }
        30 => {
            let payload: MspAttitude = MspMessage::fetch(&mut mspconn)?;
            Ok(ATTITUDE(ATTITUDE_DATA {
                time_boot_ms: 0,
                roll: (payload.angx as f64 / 10.).to_radians() as f32,
                pitch: (-payload.angy as f64 / 10.).to_radians() as f32,
                yaw: (payload.heading as f64).to_radians() as f32,
                rollspeed: 0.,
                pitchspeed: 0.,
                yawspeed: 0.,
            }))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported MAV ID {}", id),
        )),
    }
}
