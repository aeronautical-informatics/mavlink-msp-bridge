use std::collections::HashMap;
use std::collections::VecDeque;
use std::io;
use std::sync::mpsc::{self, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use log::{debug, error, info};

use crate::mavlink::WrappedMAVConnection;
use crate::msp::{MSPConnection, MSPDirection, MSPMessage, MSPVersion};
use crate::scheduler::Schedule;
use crate::Config;

pub fn event_loop(conf: &Config) {
    let mut serialport =
        serialport::open(&conf.msp_serialport).expect("unable to open serial SERIALPORT");
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

    let mavconn = WrappedMAVConnection::new(&conf.mavlink_listen);

    let mut schedule = Schedule::new(50);
    schedule.insert(1, 0u32); // insert heartbeat at 1 Hz

    info!("entering event_loop");
    loop {
        let next_task_id = schedule.next();

        match next_task_id {
            Some(id) => {
                debug!("processing task {}", id);
                // let message = generateMavmessage(id)
                // tx.send(message)
            }
            None => match mavconn.recv_timeout(Duration::from_millis(1)) {
                Ok((header, msg)) => {
                    debug!("received:\n{:?}\n", msg);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }

                Err(e) => {
                    error!("recv error: {:?}", e);
                    panic!();
                }
            },
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
