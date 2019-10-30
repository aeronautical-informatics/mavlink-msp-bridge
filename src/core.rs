use std::io;

use std::time::Duration;

use log::{debug, error, info};

use mavlink::common::MavMessage::*;

use crate::mavlink::WrappedMAVConnection;
use crate::msp::{MspConnection, MspDirection, MspMessage, MspVersion};
use crate::scheduler::Schedule;
use crate::Config;

pub fn event_loop(conf: &Config) {
    let mut mspconn =
        serialport::open(&conf.msp_serialport).expect("unable to open serial SERIALPORT");
    mspconn
        .set_timeout(Duration::from_millis(100))
        .expect("unable to set timeout for SERIALPORT");

    let msg = MspMessage {
        version: MspVersion::V2,
        direction: MspDirection::Request,
        flag: None,
        function: 102,
        payload: None,
    };

    debug!("\n{:?}\n\n{}\n", &msg, mspconn.request(&msg).unwrap());

    let mavconn = WrappedMAVConnection::new(&conf.mavlink_listen);

    let mut schedule = Schedule::new(50);
    schedule
        .insert(1, 0u32)
        .expect("unable to insert heartbeat in scheduler"); // insert heartbeat at 1 Hz

    info!("entering event_loop");
    loop {
        match schedule.next() {
            Some(id) => {
                debug!("processing task {}", id);
                // let message = generateMavmessage(id)
                // tx.send(message)
            }
            None => match mavconn.recv_timeout(Duration::from_millis(1)) {
                Ok((_header, msg)) => {
                    debug!("received:\n{:?}\n", msg);
                    match msg {
                        MESSAGE_INTERVAL(ref msg) => {
                            //schedule.insert(msg.message_rate, msg.message_id.into());
                        }
                        _ => {}
                    };
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
