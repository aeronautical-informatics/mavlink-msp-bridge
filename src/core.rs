use std::io;

use std::time::Duration;

use log::{debug, error, info, trace};

use mavlink::common::MavMessage::{self, *};

use crate::mavlink::{MavGenerator, WrappedMAVConnection};

use crate::msp::*;
use crate::scheduler::Schedule;
use crate::Config;

pub fn event_loop(conf: &Config) {
    let mut mspconn =
        serialport::open(&conf.msp_serialport).expect("unable to open serial SERIALPORT");
    mspconn
        .set_timeout(Duration::from_millis(100))
        .expect("unable to set timeout for SERIALPORT");
    mspconn
        .clear(serialport::ClearBuffer::All)
        .expect("unable to clear serial connection");
    info!("MSP connection opened on {}", mspconn.name().unwrap());

    let msg = msp_message!(MspIdent);

    let resp = mspconn.request(msg).expect("unable to receive response");
    info!("MspIdent received: {:?}", resp.payload);

    info!("waiting for MAVLink connection");
    let mavconn = WrappedMAVConnection::new(&conf.mavlink_listen);
    info!("MAVLink connection opened on {}", &conf.mavlink_listen);

    let mut schedule = Schedule::new(50);
    schedule
        .insert(1, 0)
        .expect("unable to insert heartbeat in scheduler"); // insert heartbeat at 1 Hz

    schedule.insert(5, 30); //debug
    info!("entering event_loop");
    loop {
        match schedule.next() {
            Some(id) => {
                debug!("processing task {}", id);

                let message = mspconn
                    .generate(id)
                    .expect("message could not be generated");
                mavconn.send(&message);
            }
            None => match mavconn.recv_timeout(Duration::from_millis(1)) {
                Ok((_header, msg)) => {
                    match msg {
                        HEARTBEAT(ref msg) => {}
                        MESSAGE_INTERVAL(ref msg) => {
                            let freq = (1_000_000f64 / msg.interval_us as f64) as u32;
                            schedule.insert(freq, msg.message_id.into());
                        }
                        DATA_STREAM(ref msg) => {
                            info!("gcs request {:?}", msg);
                        }
                        _ => {}
                    };
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }

                Err(e) => {
                    error!("recv error: {:?}", e);
                    break;
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
