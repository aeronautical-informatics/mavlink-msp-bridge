use std::collections::VecDeque;
use std::io;
use std::io::{Read, Write};
use std::pin::Pin;
use std::time::{Duration, Instant};
//use std::sync::Arc;

use futures::prelude::*;
use multiwii_serial_protocol_v2::{MspPacket, MspParser};
use smol::channel::{Receiver, Sender};

pub trait MspConnection: Read + Write {}
impl<T: Read + Write> MspConnection for T {}

struct MspRequestHandler {
    request: MspPacket,
    backchannel: Sender<io::Result<MspPacket>>,
    /// if timestamp is older than TTL, drop package.
    ts: Instant,
}

impl MspRequestHandler {
    pub fn new(req: MspPacket) -> Self {
        todo!();
        /*MspRequestHandler {
            request: req,

        }
        */
    }
}

/*
pub struct MspAsyncStub<T: Sized + Write, U: Sized + Read> {
    stub_tx: Sender<MspRequestHandler>,
    worker_state :Option<WorkerState<T, U>>,
}

struct WorkerState<T: Sized + Write, U: Sized + Read> {
   stub_rx: Receiver<MspRequestHandler>,
    msp_tx: T,
    msp_rx: U,
}

impl<T: Sized + Write, U: Sized+Read> MspAsyncStub<T, U> {
    /// Crates a new stub
    ///
    ///
    pub fn new(msp_tx: T, msp_rx: U, cap: usize) -> Self {
        let (stub_tx, stub_rx) = smol::channel::bounded(cap);

        MspAsyncStub {
            stub_rx,
            stub_tx,
            msp_tx,
            msp_rx,
        }
    }

    // can be called only once on the
    pub async fn spin(&mut self) ->  {
        let worker_state = self.worker_state.take().unwrap();
        let mut requested = VecDeque::with_capacity(4);

        // do requests
       let requester = smol::spawn({
           let stub = self.stub_rx.clone();
           let conn = self.msp_tx;
            async move {
                loop {
                    let request = stub.recv().await.unwrap();
                    //let serialized_message = &request.request.serialize();
                    // send request
                    requested.push_back(request);
                }
            }
        });

        let responder = smol::spawn({
            async move {
                loop {
                    // receive response via serialport
                    //let response = "";
                    //responses.push_back(response);

                    // backchannel.send(response);
                }
            }
        });

        futures::future::join(requester, responder).await;
        loop {}
        /*
        let task = some_async_fn();
        let (r, h) = task.remote_handle();
        tokio::spawn(r);
        let output = h.await;
        */
    }

    pub fn request(req: MspPacket) -> MspPacket {
        todo!();
    }
}
*/

pub fn create_stub<W: Write + Send + Sized, R: Read + Send + Sized>(
    msp_tx: W,
    msp_rx: R,
    cap: usize,
    timeout: Duration,
) -> (
    Sender<MspRequestHandler>,
    Pin<Box<dyn Future<Output = ((), ())>>>,
) {
    let (stub_tx, stub_rx) = smol::channel::bounded::<MspRequestHandler>(cap);
    let mut requested = VecDeque::with_capacity(4);
    let parser = MspParser::new();

    // do requests
    let requester = smol::spawn({
        async move {
            loop {
                let rh = stub_rx.recv().await.unwrap();
                let mut buf = Vec::with_capacity(rh.request.packet_size_bytes_v2());
                rh.request.serialize_v2(&mut buf);
                requested.push_back(rh);
                msp_tx.write(&buf); // TODO make me async
            }
        }
    });

    let responder = smol::spawn({
        async move {
            let mut buf = Vec::with_capacity(1024);
            loop {
                msp_rx.read(&mut buf); // TODO make me async

                for byte in &buf {
                    match parser.parse(*byte) {
                        Ok(Some(msp_response)) => {
                            requested
                                .iter()
                                .filter(|rh| rh.ts.elapsed() <= timeout)
                                .for_each(|rh| {
                                    if rh.request.cmd == msp_response.cmd {
                                        rh.backchannel.send(Ok(msp_response));
                                    }
                                });
                            // Delete request once we responded
                        }
                        _ => {}
                    }
                }
                buf.clear();

                // remove all timed out messages.
                while requested
                    .front()
                    .map_or(false, |rh| rh.ts.elapsed() > timeout)
                {
                    match requested.pop_front() {
                        Some(rh) => {
                            rh.backchannel.send(todo!());
                            // TODO respond with error
                        }
                        None => {}
                    }
                }

                // receive response via serialport
                //let response = "";
                //responses.push_back(response);

                // backchannel.send(response);
            }
        }
    });

    (stub_tx, futures::future::join(requester, responder).boxed())
}
