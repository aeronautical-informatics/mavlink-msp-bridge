use std::env;
use std::net::{SocketAddr, ToSocketAddrs};

use clap::{value_t_or_exit, App, Arg};
use log::{debug, info};
use serialport::available_ports;

mod core;
mod mavlink;
mod msp;
mod serial;

enum TransportLayer {
    UDP,
    TCP,
}

#[derive(Debug)]
pub struct Config {
    mavlink_listen: String,
    msp_serialport: String,
    msp_baud: u32,
}

fn main() {
    env::set_var("RUST_LOG", "debug");
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
                .default_value("9600"),
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
        mavlink_listen: matches.value_of("mavlink").unwrap().to_string(),
        msp_serialport: matches
            .value_of("serial")
            .unwrap_or(&available_ports().expect("No serial port")[0].port_name)
            .to_string(),

        msp_baud: value_t_or_exit!(matches.value_of("baud"), u32),
    };

    info!("started");
    debug!("{:?}", &conf);
    core::event_loop(&conf);

    //   mavlink::get_connection(&conf);
}
