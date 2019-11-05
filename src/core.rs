use std::io;

use std::time::{Duration, Instant};

use log::{debug, error, info, trace};

use mavlink::common::MavMessage::{self, *};

use crate::mavlink::{generate, WrappedMAVConnection};

use crate::msp::*;
use crate::scheduler::Schedule;
use crate::Config;

pub fn event_loop(conf: &Config) {
    // initializes stratum null
    let t0 = Instant::now();

    // initializes the MSP connection
    let mut mspconn =
        serialport::open(&conf.msp_serialport).expect("unable to open serial SERIALPORT");
    mspconn
        .set_timeout(Duration::from_millis(100))
        .expect("unable to set timeout for SERIALPORT");
    mspconn
        .clear(serialport::ClearBuffer::All)
        .expect("unable to clear serial connection");

    // testing wether MSP connection is attached to MSP FC
    let resp: MspIdent = MspMessage::fetch(&mut mspconn).expect("unable to receive response");
    debug!("MspIdent received {:?}", resp);
    info!("MSP connection opened on {}", mspconn.name().unwrap());

    // initializes MAV connection
    info!("waiting for MAVLink connection");
    let mavconn = WrappedMAVConnection::new(&conf);
    info!("MAVLink connection opened on {}", &conf.mavlink_listen);

    // initializes scheduler and inserts HEARTBEAT task
    let mut schedule = Schedule::new(50);
    schedule
        .insert(1, 0)
        .expect("unable to insert heartbeat in scheduler");

    schedule.insert(5, 30); //debug

    // enters eventloop to process scheduled messages and incoming messages
    info!("entering event_loop");
    loop {
        match schedule.next() {
            // some MAV message is scheduled to be sent now
            Some(id) => {
                trace!("processing task {}", id);
                let message = generate(&mut mspconn, id).expect("message could not be generated");
                mavconn.send(&message);
            }
            // no scheduled MAV message, checking for incoming MAV message
            None => match mavconn.recv_timeout(Duration::from_millis(1)) {
                Ok((_header, msg)) => {
                    match msg {
                        HEARTBEAT(ref msg) => {}
                        PARAM_REQUEST_LIST(ref msg) => {
                            let message =
                                generate(&mut mspconn, 22).expect("message could not be generated");
                            mavconn.send(&message);
                        }
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
