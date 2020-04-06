use std::io;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use log::{debug, error};
use mavlink::common::*;
use mavlink::{MavConnection, MavHeader};

use crate::Config;

type MavResponse = io::Result<(MavHeader, MavMessage)>;

pub struct WrappedMavConnection {
    system_id: u8,
    rx: mpsc::Receiver<MavResponse>,
    conn: Arc<dyn MavConnection<mavlink::common::MavMessage> + Send + Sync>,
}

impl WrappedMavConnection {
    pub fn new(conf: &Config) -> Self {
        let mavconn: Arc<dyn MavConnection<mavlink::common::MavMessage> + Send + Sync> =
            mavlink::connect(&conf.mavlink_listen).unwrap().into();

        let (tx, rx) = mpsc::channel();
        thread::spawn({
            let mavconn = mavconn.clone();
            move || loop {
                tx.send(mavconn.recv())
                    .expect("broken MPSC in WrappedMAVConnection");
            }
        });

        Self {
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
        let mut header = mavlink::MavHeader::default();
        header.system_id = self.system_id;
        debug!("sending: {:?}", data);
        self.conn.send(&header, data)
    }
}
