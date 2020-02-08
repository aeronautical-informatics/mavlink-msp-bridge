use std::io;

use mavlink::common::MavMessage::*;
use mavlink::common::*;

use crate::msp::*;
use crate::Config;

pub fn heartbeat(
    _conf: &Config,
    _mspconn: &mut dyn MspConnection,
    _context: Option<&MavMessage>,
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
    _conf: &Config,
    _mspconn: &mut dyn MspConnection,
    _context: Option<&MavMessage>,
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
    _context: Option<&MavMessage>,
) -> io::Result<MavMessage> {
    let payload: MspRawImu = MspMessage::fetch(&mut mspconn)?;
    return Ok(RAW_IMU(RAW_IMU_DATA {
        time_usec: conf.t0.elapsed().as_micros() as u64,
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
    _context: Option<&MavMessage>,
) -> io::Result<MavMessage> {
    let payload: MspAttitude = MspMessage::fetch(&mut mspconn)?;
    Ok(ATTITUDE(ATTITUDE_DATA {
        time_boot_ms: conf.t0.elapsed().as_millis() as u32,
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
