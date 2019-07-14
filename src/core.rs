use std::thread;
use std::sync::mpsc::{self, TryRecvError};
use std::collections::VecDeque;


use futures::sink::Sink;
use futures::stream::Stream;
use tokio;
use tokio::codec::{Framed, LinesCodec};
use tokio_serial;

use crate::Config;
use crate::msp::{MSPCodec, MSPMessage};


struct Translation {
    mavlink_request: String,
    msp_requests: Vec<MSPMessage>,
}

pub fn event_loop(conf: &Config){
    let (msp_tx, msp_rx) = mpsc::channel();
    let (core_tx, core_rx) = mpsc::channel();

    let mut msp_thread = thread::spawn(move || {

        let settings = tokio_serial::SerialPortSettings::default();
        let mut port = tokio_serial::Serial::from_path(&conf.msp_serialport, &settings).unwrap();
        
        #[cfg(unix)]
        port.set_exclusive(false)
            .expect("Unable to set serial port exlusive");
        
        let framed_sock = Framed::new(port, MSPCodec::new());
        
        loop {
            let mut index = 0;
            core_tx.send("MSP Thread started".to_string());

            // process work
            let mut translation: Translation = match msp_rx.recv() {
                Ok(t) => t,
                Err(e) =>panic!("{:?}",e)
            };

            for req in translation.msp_requests.iter() {
                framed_sock.send(*req);
            }

            let fetcher = framed_sock
            .for_each(|message| {
                println!("{:?}", message);
                Ok(())
            });
        }
    });

//    let mut mavlink_threads : Vec<thread> = Vec::new();
//    
//    mavlink_threads.push(
//    thread::spawn(move || {
//        core_tx.send("MSP Thread started".to_string());
//        loop {
//            // process work
//            let mut current_job : Translation= match msp_rx.recv() {
//                Ok(job) => job,
//                Err(e) =>panic!("{:?}",e)
//            };
//
//            for req  in current_job.msp_requests.iter() {
//                //send job,fetch job
//            }
//        }
//    }));

    loop{
        let msg: String = core_rx.recv().unwrap(); //ugly
        println!("{}", msg);
    }
}
