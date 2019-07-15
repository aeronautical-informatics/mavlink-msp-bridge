use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use log::{debug, error, info};

use crate::msp::{MSPConnection, MSPDirection, MSPMessage, MSPVersion};
use crate::Config;

struct Translation {
    mavlink_request: String,
    msp_requests: Vec<MSPMessage>,
}

struct MessageInterval {
    id: u32,
    interval: Duration,
    last: Instant,
}

impl MessageInterval {
    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last) >= self.interval {
            self.last = now;
            return true;
        }
        false
    }

    pub fn new(stream_id: u32, freq: u16) -> Self {
        let now = Instant::now();
        Self {
            id: stream_id,
            interval: Duration::from_nanos(1_000_000_000 / freq as u64),
            last: now,
        }
    }
}

pub fn event_loop(conf: &Config) {
    let mut serialport = serialport::open(&conf.msp_serialport).unwrap();
    serialport.set_timeout(Duration::from_millis(100));
    let mut mspconn = MSPConnection::new(serialport);

    let msg = MSPMessage {
        version: MSPVersion::V2,
        direction: MSPDirection::Request,
        flag: None,
        function: 102,
        payload: "".as_bytes().to_vec(),
    };

    debug!("\n{:?}\n\n{}\n", &msg, mspconn.request(&msg).unwrap());

    info!("entering event_loop");
    loop {
        debug!("waiting for MAVLink connection");
        let mut mavconn = mavlink::connect(&conf.mavlink_listen).unwrap();
        debug!("MAVLink connection received");
        //mavconn.set_protocol_version(mavlink::MavlinkVersion::V2);

        let mavconn = Arc::new(mavconn);

        //gcs.send(&mavlink::MavHeader::get_default_header(), &request_stream())
        //   .unwrap();

        let (tx, rx) = mpsc::channel();
        thread::spawn({
            let mavconn = mavconn.clone();
            move || loop {
                tx.send(mavconn.recv());
            }
        });

        let mut streams = HashMap::new();
        streams.insert(0, MessageInterval::new(0, 1));

        loop {
            for s in streams.values_mut() {
                if s.check() {
                    let msg = mspconn
                        .generate_mav_message(s.id)
                        .expect("unable to generate needed mavlink message");
                    debug!("sent: \n{:?}", msg);
                    mavconn
                        .send_default(&msg)
                        .expect("unable to send mavlink message");
                }
            }
            match rx.try_recv() {
                Ok(Ok((header, msg))) => {
                    debug!("received:\n{:?}\n{:?}\n", header, msg);
                    //match msg {
                    //    mavlink::common::
                    //}
                }
                Ok(Err(e)) => match e.kind() {
                    std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_secs(1));
                        continue;
                    }
                    _ => {
                        println!("recv error: {:?}", e);
                        break;
                    }
                },
                _ => {}
            }
        }
    }
}

pub fn request_parameters() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::PARAM_REQUEST_LIST(mavlink::common::PARAM_REQUEST_LIST_DATA {
        target_system: 0,
        target_component: 0,
    })
}

pub fn request_stream() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::REQUEST_DATA_STREAM(mavlink::common::REQUEST_DATA_STREAM_DATA {
        target_system: 0,
        target_component: 0,
        req_stream_id: 0,
        req_message_rate: 10,
        start_stop: 1,
    })
}
