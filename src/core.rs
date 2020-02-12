use std::collections::HashMap;
use std::io;
use std::time::Duration;

use log::{debug, error, info, trace, warn};

use mavlink::common::*;

use crate::mavlink::WrappedMavConnection;

use crate::msp::*;
use crate::scheduler::Schedule;
use crate::translator::*;
use crate::Config;

pub fn event_loop(conf: &Config) {
    // initializes the MSP connection
    let mut mspconn =
        serialport::open(&conf.msp_serialport).expect("unable to open serial SERIALPORT");
    mspconn
        .set_timeout(Duration::from_millis(100))
        .expect("unable to set timeout for SERIALPORT");
    mspconn
        .clear(serialport::ClearBuffer::All)
        .expect("unable to clear serial connection");

    let mut generators: HashMap<
        u32,
        fn(
            conf: &Config,
            mspconn: &mut dyn MspConnection,
            context: Option<&MavMessage>,
        ) -> io::Result<MavMessage>,
    > = HashMap::new();

    let mut _response_to: HashMap<u32, u32> = HashMap::new();

    generators.insert(0, heartbeat);
    generators.insert(22, param_value);
    generators.insert(27, raw_imu);
    generators.insert(30, attitude);
    //generators.insert(44, mission_count);

    // testing wether MSP connection is attached to MSP FC
    let resp: MspIdent = MspMessage::fetch(&mut mspconn).expect("unable to receive response");
    debug!("MspIdent received {:?}", resp);
    info!("MSP connection opened on {}", mspconn.name().unwrap());

    // initializes MAV connection
    info!("waiting for MAVLink connection");
    let mavconn = WrappedMavConnection::new(&conf);
    info!("MAVLink connection opened on {}", &conf.mavlink_listen);

    // initializes scheduler and inserts HEARTBEAT task
    let mut schedule = Schedule::new(50);
    schedule
        .insert(1, 0)
        .expect("unable to insert heartbeat in scheduler");

    // inform about attitude on high frequency
    schedule.insert(30, 30).unwrap();

    // enters eventloop to process scheduled messages and incoming messages
    info!("entering event_loop");
    loop {
        match schedule.next() {
            // some MAV message is scheduled to be sent now
            Some(id) => {
                trace!("processing task {}", id);
                if let Some(generator) = generators.get(&id) {
                    let message = generator(&conf, &mut mspconn, None)
                        .expect("message could not be generated");
                    let _ = mavconn.send(&message);
                } else {
                    warn!("cannot process subscription for task {}", id);
                }
            }
            // no scheduled MAV message, checking for incoming MAV message
            None => match mavconn.recv_timeout(Duration::from_millis(1)) {
                Ok((_header, msg)) => {
                    match msg {
                        MavMessage::HEARTBEAT(ref _msg) => {}
                        MavMessage::MESSAGE_INTERVAL(ref msg) => {
                            let freq = (1_000_000f64 / msg.interval_us as f64) as u32;
                            schedule.insert(freq, msg.message_id.into()).unwrap();
                        }
                        msg => {
                            warn!("received MavMessage, don't know what to do: {:?}", msg);
                        }
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
