use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{App, Error, Handler, HttpRequest, HttpResponse, HttpServer, Responder, web};
use actix_web::middleware::Logger;
use actix_web_actors::ws;
use actix_web_actors::ws::{Message, ProtocolError};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);

/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);
#[derive(Debug)]
pub struct WsSession {
    /// unique session id
    pub id: usize,

    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    pub hb: Instant,
    pub appContext: Arc<AppContext>,
}

impl WsSession {
    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
        self.appContext.counter.fetch_add(1,Ordering::SeqCst);
    }
    fn stopped(&mut self, ctx: &mut Self::Context) {
        log::debug!("ws actor stopped ");
        self.appContext.counter.fetch_sub(1,Ordering::SeqCst);

    }
}

// websocket message handle
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };
        log::info!("WEBSOCKET MESSAGE: {:?}", msg);

        match msg {
            Message::Text(_) => {
            }
            Message::Binary(_) => {
                println!("Unexpected binary")
            }
            Message::Continuation(_) => {
                ctx.stop();
            }
            Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Message::Pong(_) => {
                self.hb = Instant::now();
            }
            Message::Close(reason) => {
                ctx.close(reason);
                ctx.stop();
            }
            Message::Nop => {}
        }
    }
}


// update http to websocket
async fn ws_route(
    req: HttpRequest,
    stream: web::Payload,
    appCtx: web::Data<Arc<AppContext>>
) -> Result<HttpResponse, Error> {
    ws::start(
        WsSession { id: 0, hb: Instant::now(),appContext: appCtx.get_ref().clone() },
        &req,
        stream,
    )
}
/// Displays state
async fn get_count(count: web::Data<Arc<AppContext>>) -> impl Responder {
    let current_count = count.counter.load(Ordering::SeqCst);
    format!("Visitors: {}", current_count)
}
// This struct represents state
#[derive(Debug)]
pub struct AppContext {
    counter: AtomicUsize,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));

    // set up applications state
    // keep a count of the number of visitors
    let app_state = Arc::new(AppContext{counter:AtomicUsize::new(0)});


    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .route("/ws", web::get().to(ws_route))
            .route("/count", web::get().to(get_count))
            .wrap(Logger::default())
    })
        .workers(2)
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

