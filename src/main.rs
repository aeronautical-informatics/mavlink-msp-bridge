use std::env;
use std::time::Instant;

use clap::{value_t_or_exit, App, Arg};
use log::{debug, error, info};
use serialport::available_ports;

#[macro_use]
mod msp;
mod core;
mod mavlink;
mod scheduler;
mod serial;

#[derive(Debug)]
pub struct Config {
    t0: Instant,
    mavlink_listen: String,
    mavlink_system_id: u8,
    msp_serialport: String,
    msp_baud: u32,
}

fn main() {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or("debug".to_string()),
    );
    env_logger::init();
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("serial")
                .value_name("SERIALPORT")
                .help("Select serial port for MSP side"),
        )
        .arg(
            Arg::with_name("baud")
                .short("b")
                .long("baud")
                .help("Baud rate for the serial port")
                .default_value("115200"),
        )
        .arg(
            Arg::with_name("mavlink")
                .short("m")
                .long("mavlink-listen")
                .value_name("transport:ip:port")
                .help("Select mavlink connection adress")
                .default_value("tcpin:0.0.0.0:5760"),
        )
        .arg(
            Arg::with_name("mavlink-system-id")
                .short("i")
                .long("mavlink-system-id")
                .help("sytemd_id to use for the virtual drone")
                .default_value("1"),
        )
        .arg(
            Arg::with_name("list-serialports")
                .short("l")
                .long("list-serial")
                .help("Lists all available serialports"),
        )
        .get_matches();

    if matches.is_present("list-serialports") {
        serial::list_serialports();
        return;
    }

    let conf = Config {
        t0: Instant::now(),
        mavlink_listen: matches.value_of("mavlink").unwrap().to_string(),
        msp_serialport: matches
            .value_of("serial")
            .unwrap_or(match available_ports() {
                Ok(ref a) if a.len() >= 1 => &a[0].port_name,
                _ => {
                    error!("no serialport found");
                    panic!();
                }
            })
            .to_string(),
        msp_baud: value_t_or_exit!(matches.value_of("baud"), u32),
        mavlink_system_id: value_t_or_exit!(matches.value_of("mavlink-system-id"), u8),
    };

    info!("started");
    debug!("{:?}", &conf);
    core::event_loop(&conf);
}
