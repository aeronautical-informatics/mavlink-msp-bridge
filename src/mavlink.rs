use std::io;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use mavlink::common::MavMessage;
use mavlink::{MavConnection, MavHeader};

use log::{debug, error, info};

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
