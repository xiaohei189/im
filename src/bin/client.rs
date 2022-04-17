//! Simple websocket client.
use std::sync::Arc;
use std::time::Duration;
use std::{io, thread};

use actix_web::web::Bytes;
use awc::ws;
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::{select, sync::mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[actix_web::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting echo WebSocket client");

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let mut cmd_rx = UnboundedReceiverStream::new(cmd_rx);
    let tx = Arc::new(cmd_tx);
    //
    // // run blocking terminal input reader on separate thread
    let cmd_tx = Arc::clone(&tx);
    let input_thread = thread::spawn(move || loop {
        let mut cmd = String::with_capacity(32);

        if io::stdin().read_line(&mut cmd).is_err() {
            log::error!("error reading line");
            return;
        }

        cmd_tx.send(cmd).unwrap();
    });

    //  heartbeat
    let ping_tx = Arc::clone(&tx);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(5));
        ping_tx.send(String::from("ping")).unwrap();
    });

    // create 10 thread
    let (res, mut ws) = awc::Client::new()
        .ws("ws://127.0.0.1:8080/ws")
        .connect()
        .await
        .unwrap();

    log::debug!("response: {res:?}");
    log::info!("connected; server will echo messages sent");
    loop {
        select! {
            Some(msg) = ws.next() => {
                match msg {
                    Ok(ws::Frame::Text(txt)) => {
                        // log echoed messages from server
                        log::info!("Server: {:?}", txt)
                    }


                    _ => {}
                }
            }

            Some(cmd) = cmd_rx.next() => {
                if cmd.is_empty() {
                    continue;
                }

                if cmd=="ping"{
                    ws.send(ws::Message::Ping(Bytes::new())).await.unwrap();

                }else {
                    ws.send(ws::Message::Text(cmd.into())).await.unwrap();
                }

            }

            else => break
        }
    }
    input_thread.join().unwrap();
}
