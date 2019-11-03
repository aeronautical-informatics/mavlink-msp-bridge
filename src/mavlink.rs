use std::io;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mavlink::common::MavMessage::*;
use mavlink::common::*;
use mavlink::{MavConnection, MavHeader};

use log::{debug, error, trace};

use crate::msp::*;

type MavResponse = io::Result<(MavHeader, MavMessage)>;

pub struct WrappedMAVConnection {
    rx: mpsc::Receiver<MavResponse>,
    conn: Arc<dyn MavConnection + Send + Sync>,
}

impl WrappedMAVConnection {
    pub fn new(mavlink_listen: &str) -> Self {
        let mavconn: Arc<dyn MavConnection + Send + Sync> =
            mavlink::connect(mavlink_listen).unwrap().into();

        let (tx, rx) = mpsc::channel();
        thread::spawn({
            let mavconn = mavconn.clone();
            move || loop {
                tx.send(mavconn.recv());
            }
        });

        WrappedMAVConnection {
            rx: rx,
            conn: mavconn,
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> MavResponse {
        match self.rx.recv_timeout(timeout) {
            Ok(mav_response) => {
                trace!("received:\n{:?}\n", mav_response);
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
        self.conn.send_default(data)
    }
}

pub trait MavGenerator {
    fn can_generate(message_id: u32) -> bool;
    fn generate(&mut self, id: u32) -> io::Result<MavMessage>;
}

impl<T: MspConnection> MavGenerator for T {
    fn can_generate(message_id: u32) -> bool {
        false
    }

    fn generate(&mut self, id: u32) -> io::Result<MavMessage> {
        match id {
            0 => {
                Ok(HEARTBEAT(HEARTBEAT_DATA {
                    custom_mode: 0,
                    mavtype: MavType::MAV_TYPE_GENERIC,
                    autopilot: MavAutopilot::MAV_AUTOPILOT_GENERIC_WAYPOINTS_AND_SIMPLE_NAVIGATION_ONLY,
                    base_mode: MavModeFlag::empty(),
                    system_status: MavState::MAV_STATE_STANDBY,
                    mavlink_version: 0x3,
                }))
            }
            27 => {
                if let MspPayload::MspRawImu(payload) =
                    self.request(msp_message!(MspRawImu))?.payload
                {
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
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "wrong MSP payload",
                    ))
                }
            }
            30 => {
                if let MspPayload::MspAttitude(payload) =
                    self.request(msp_message!(MspAttitude))?.payload
                {
                    Ok(ATTITUDE(ATTITUDE_DATA {
                        time_boot_ms: 0,
                        roll: (payload.angx as f64 / 10.).to_radians() as f32,
                        pitch: (-payload.angy as f64 / 10.).to_radians() as f32,
                        yaw: (payload.heading as f64).to_radians() as f32,
                        rollspeed: 0.,
                        pitchspeed: 0.,
                        yawspeed: 0.,
                    }))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "wrong MSP payload",
                    ))
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported MAV ID {}", id),
            )),
        }
    }
}
