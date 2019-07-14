use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::Config;

pub fn get_connection(conf: &Config) {
    println!("here0");
    let mut mavconn = mavlink::connect(&conf.mavlink_listen).unwrap();
    
    println!("here1");
    let vehicle = Arc::new(mavconn);

    println!("here2");

    vehicle
        .send(
            &mavlink::MavHeader::get_default_header(),
            &request_parameters(),
        )
        .unwrap();
    vehicle
        .send(&mavlink::MavHeader::get_default_header(), &request_stream())
        .unwrap();

    thread::spawn({
        let vehicle = vehicle.clone();
        move || loop {
            let res = vehicle.send_default(&heartbeat_message());
            if ! res.is_ok() {
                println!("send failed: {:?}", res);
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    loop {
        match vehicle.recv() {
            Ok((_header, msg)) => {
                println!("received: {:?}", msg);
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_secs(1));
                    continue;
                }
                _ => {
                    println!("recv error: {:?}", e);
                }
            },
        }
    }
}

pub fn heartbeat_message() -> mavlink::common::MavMessage {
    mavlink::common::MavMessage::HEARTBEAT(mavlink::common::HEARTBEAT_DATA {
        custom_mode: 0,
        mavtype: mavlink::common::MavType::MAV_TYPE_QUADROTOR,
        autopilot: mavlink::common::MavAutopilot::MAV_AUTOPILOT_ARDUPILOTMEGA,
        base_mode: mavlink::common::MavModeFlag::empty(),
        system_status: mavlink::common::MavState::MAV_STATE_STANDBY,
        mavlink_version: 0x3,
    })
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
