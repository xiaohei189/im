//! Simple websocket client.
use std::{io, thread};
use std::time::{Duration, Instant};
use websocket::{ClientBuilder, OwnedMessage};
use websocket::r#async::client::{Client, ClientNew};
use websocket::r#async::TcpStream;
use websocket::futures::{Future, Stream, Sink};
use websocket::Message;
use tokio::runtime::Builder;
use tokio::sync::mpsc;
use tokio::time;
use tokio::time::Interval;
use tokio_stream::wrappers::UnboundedReceiverStream;
#[tokio::main]

async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    log::info!("starting echo WebSocket client");

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let mut cmd_rx = UnboundedReceiverStream::new(cmd_rx);
    //
    // // run blocking terminal input reader on separate thread
    let input_thread = thread::spawn(move || loop {
        let mut cmd = String::with_capacity(32);

        if io::stdin().read_line(&mut cmd).is_err() {
            log::error!("error reading line");
            return;
        }

        cmd_tx.send(cmd).unwrap();
    });

    for i in 0..15000 {
        log::info!("create client {}",i);

        let mut client = ClientBuilder::new("ws://127.0.0.1:8080/ws").unwrap().connect_insecure().unwrap();

        client.incoming_messages()

        tokio::spawn(async move {
            let when = Instant::now() ;
            let mut interval = time::interval(time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let message = OwnedMessage::Ping(String::from("ping").into_bytes());
                client.send_message(&message);
            }
        });

        // let (res, mut ws) = awc::Client::new()
        //     .ws("ws://127.0.0.1:8080/ws")
        //     .connect()
        //     .await
        //     .unwrap();

        // log::debug!("response: {res:?}");
        // log::info!("connected; server will echo messages sent");
        // let timer = timer::Timer::new();
        // tokio::spawn(async move {
        //     ws.send(ws::Message::Ping(Bytes::new())).await;
        // });
        // loop {
        //     select! {
        //         Some(msg) = ws.next() => {
        //             match msg {
        //                 Ok(ws::Frame::Text(txt)) => {
        //                     // log echoed messages from server
        //                     log::info!("Server: {:?}", txt)
        //                 }
        //
        //                 Ok(ws::Frame::Ping(_)) => {
        //                     // respond to ping probes
        //                     ws.send(ws::Message::Pong(Bytes::new())).await.unwrap();
        //                 }
        //
        //                 _ => {}
        //             }
        //         }
        //
        //         Some(cmd) = cmd_rx.next() => {
        //             if cmd.is_empty() {
        //                 continue;
        //             }
        //
        //             ws.send(ws::Message::Text(cmd.into())).await.unwrap();
        //         }
        //
        //         else => break
        //     }
        // };
    }
    input_thread.join().unwrap();
}