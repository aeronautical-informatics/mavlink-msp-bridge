use std::io;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mavlink::common::MavMessage::*;
use mavlink::common::*;
use mavlink::{MavConnection, MavHeader};

use log::{debug, error};

use crate::msp::MspConnection;

type MavResponse = io::Result<(MavHeader, MavMessage)>;

pub struct WrappedMAVConnection {
    rx: mpsc::Receiver<MavResponse>,
    conn: Arc<dyn MavConnection + Send + Sync>,
}

impl WrappedMAVConnection {
    pub fn new(mavlink_listen: &str) -> Self {
        debug!("waiting for MAVLink connection");
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
                debug!("received:\n{:?}\n", mav_response);
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
}

pub struct MavGenerator {}

impl MavGenerator {
    pub fn is_supported_message(message_id: u32) -> bool {
        false
    }

    pub fn get_mav_message(message_id: u32, mspconn: &dyn MspConnection) -> Option<MavMessage> {
        match message_id {
            0 => Some(HEARTBEAT(HEARTBEAT_DATA {
                custom_mode: 0,
                mavtype: MavType::MAV_TYPE_QUADROTOR,
                autopilot: MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
                base_mode: MavModeFlag::empty(),
                system_status: MavState::MAV_STATE_STANDBY,
                mavlink_version: 0x3,
            })),

            _ => None,
        }
    }
}
