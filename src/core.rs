use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use log::{debug, error, info, trace, warn};

use mavlink::common::MavMessage::*;
use mavlink::common::*;

use crate::mavlink::WrappedMavConnection;

use crate::msp::*;
use crate::scheduler::Schedule;
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

    let mut response_to: HashMap<u32, u32> = HashMap::new();

    generators.insert(0, heartbeat);
    generators.insert(22, param_value);
    generators.insert(27, raw_imu);
    generators.insert(30, attitude);
    //  generators.insert(44, mission_count);

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

    schedule.insert(5, 30); //debug

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
                    mavconn.send(&message);
                } else {
                    warn!("cannot process subscription for task {}", id);
                }
            }
            // no scheduled MAV message, checking for incoming MAV message
            None => match mavconn.recv_timeout(Duration::from_millis(1)) {
                Ok((_header, msg)) => {
                    match msg {
                        MavMessage::HEARTBEAT(ref msg) => {}
                        MavMessage::MESSAGE_INTERVAL(ref msg) => {
                            warn!("HOORAY");
                            let freq = (1_000_000f64 / msg.interval_us as f64) as u32;
                            schedule.insert(freq, msg.message_id.into());
                        }
                        MavMessage::DATA_STREAM(ref msg) => {
                            warn!("HOORAY");
                            info!("gcs request {:?}", msg);
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

pub fn heartbeat(
    conf: &Config,
    mut mspconn: &mut dyn MspConnection,
    context: Option<&MavMessage>,
) -> io::Result<MavMessage> {
    Ok(HEARTBEAT(HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: MavType::MAV_TYPE_GENERIC,
        autopilot: MavAutopilot::MAV_AUTOPILOT_GENERIC_WAYPOINTS_AND_SIMPLE_NAVIGATION_ONLY,
        base_mode: MavModeFlag::empty(),
        system_status: MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    }))
}

pub fn param_value(
    conf: &Config,
    mut mspconn: &mut dyn MspConnection,
    context: Option<&MavMessage>,
) -> io::Result<MavMessage> {
    Ok(PARAM_VALUE(PARAM_VALUE_DATA {
        param_value: 0.,
        param_count: 0,
        param_index: 0,
        param_id: [
            ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ', ' ',
        ],
        param_type: MavParamType::MAV_PARAM_TYPE_UINT8,
    }))
}

pub fn raw_imu(
    conf: &Config,
    mut mspconn: &mut dyn MspConnection,
    context: Option<&MavMessage>,
) -> io::Result<MavMessage> {
    let payload: MspRawImu = MspMessage::fetch(&mut mspconn)?;
    return Ok(RAW_IMU(RAW_IMU_DATA {
        time_usec: 0, //t0.elapsed().as_micros() as u64,
        xacc: payload.accx,
        yacc: payload.accy,
        zacc: payload.accz,
        xgyro: payload.gyrx,
        ygyro: payload.gyry,
        zgyro: payload.gyrz,
        xmag: payload.magx,
        ymag: payload.magy,
        zmag: payload.magz,
    }));
}
pub fn attitude(
    conf: &Config,
    mut mspconn: &mut dyn MspConnection,
    context: Option<&MavMessage>,
) -> io::Result<MavMessage> {
    let payload: MspAttitude = MspMessage::fetch(&mut mspconn)?;
    Ok(ATTITUDE(ATTITUDE_DATA {
        time_boot_ms: 0,
        roll: (payload.angx as f64 / 10.).to_radians() as f32,
        pitch: (-payload.angy as f64 / 10.).to_radians() as f32,
        yaw: (payload.heading as f64).to_radians() as f32,
        rollspeed: 0.,
        pitchspeed: 0.,
        yawspeed: 0.,
    }))
}

//pub fn mission_count(
//    conf: &Config,
//    mut mspconn: &mut dyn MspConnection,
//    id: u32,
//    context: Option<&MavMessage>,
//) -> io::Result<MavMessage> {
//    use std::convert::TryInto;
//         let msg:MISSION_REQUEST_DATA  = context.unwrap().try_into().unwrap();
//        Ok(MISSION_COUNT(MISSION_COUNT_DATA {
//            target_system: msg.target_system,
//            target_component: msg.target_component,
//            count: 0,
//        }))
//
//}
