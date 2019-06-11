use std::net::{SocketAddr, ToSocketAddrs};

use clap::{App, Arg};
use log::info;

mod msp;
mod serial;

struct Config {
    mavlink_listen: String,
    msp_serialport: String,
    msp_baud: u32,
}

fn main() {
    env_logger::init();
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("serial")
                .short("s")
                .value_name("SERIALPORT")
                .help("Select serial port for MSP side. Defaults to the first serial port found.")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("list-serialports")
                .short("l")
                .long("list-serial")
                .help("Lists all available serialports"),
        )
        .arg(
            Arg::with_name("baud")
                .short("b")
                .long("baud")
                .help("The baud rate to connect at"),
        )
        .arg(
            Arg::with_name("mavlink-listen")
                .short("m")
                .long("mavlink")
                .value_name("ip:port")
                .help("Select mavlink listen adress")
                .default_value("0.0.0.0:5760"),
        )
        .get_matches();

    if matches.is_present("list-serialports") {
        serial::list_serialports();
        return;
    }

    let conf = Config {
        mavlink_listen: matches.value_of("mavlink").unwrap().to_string(),
        msp_serialport: matches.value_of("serialport").unwrap().to_string(),
        msp_baud: 9600,
    };

    info!("started");
}
//wucke13: write a `tokio::codec::Decoder` impl and ensure `AsyncRead` is available for your serial port, then construct a stream using `FramedRead`
