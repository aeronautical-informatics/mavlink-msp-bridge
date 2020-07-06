//! A communication bridge to masquerade a MSP capable drone as MAVLink drone.

#![deny(unsafe_code)]
#![warn(missing_docs)]

#[macro_use]
extern crate log;

use std::env;
use std::time::Instant;

use clap::Clap;

mod core;
mod msp;
mod scheduler;
mod translator;

/// A communication bridge to masquerade a MSP capable drone as MAVLink drone.
///
/// The translation is described as a map of functions. A MAVLink message ID maps to a generator
/// functions. Said generator functions receive a reference to the static Config, a reference to
/// the MSP stub to poll information from the actual drone and an optional context, consisting of
/// the logically previous MAVLink message.
#[derive(Clone, Debug, Clap)]
#[clap(version, author, about)]
pub struct Config {
    /// MAVLink listen address. Can be TCP/UDP/Serialport/File. For further information, see
    /// https://docs.rs/mavlink/0/mavlink/fn.connect.html
    #[clap(short = "l", long, default_value = "udpbcast:0.0.0.0:14550")]
    mavlink_listen: String,

    /// MAVLink system id of masked drone.
    #[clap(short = "i", long, default_value = "1")]
    mavlink_system_id: u8,

    /// serialport to MSP FC
    #[clap(short = "s", long = "serial")]
    msp_serialport: String,

    /// baudrate for given serialport
    #[clap(short = "b", long = "baud", default_value = "115200")]
    msp_baud: u32,

    /// time zero
    #[clap(skip = Instant::now())]
    t0: Instant,
}

fn main() {
    env::set_var(
        "RUST_LOG",
        env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
    );

    let conf = Config::parse();

    info!("started");
    debug!("{:?}", &conf);
    core::event_loop(&conf);
}
