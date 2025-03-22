use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use flume::{Receiver, Sender};
use log::info;
use serde::{Deserialize, Serialize};
use Yggdrasil::{CONNECTED_CLIENTS, USERNAME_UUID_MAP};

struct WebSocketSession {
    id: String,
    username: Option<String>,
    connected_to: Option<String>,
    state: SessionState,
    sender: Sender<String>,
    receiver: Receiver<String>,
}

enum SessionState {
    AwaitingUsername,
    AwaitingRecipient,
    AwaitingRecipientChoice(String),
    Ready,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        info!("New Client Connected");

        let receiver = self.receiver.clone();
        let addr = ctx.address();
        println!("WebSocket session uuid: {}", self.id);

        CONNECTED_CLIENTS.insert(self.id.clone(), self.sender.clone());
        tokio::spawn(async move {
            println!("WebSocket session started");

            while let Ok(msg) = dbg!(receiver.recv_async().await) {
                println!("Received message: {}", msg);
                addr.do_send(WsMessage(msg));
            }
            println!("WebSocket session ended");
        });
    }
    fn stopped(&mut self, _ctx: &mut Self::Context) {
        CONNECTED_CLIENTS.remove(&self.id);

        println!("WebSocket session stopped");
    }
}

struct WsMessage(String);

impl actix::Message for WsMessage {
    type Result = ();
}

impl actix::Handler<WsMessage> for WebSocketSession {
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => match &self.state {
                SessionState::AwaitingUsername => {
                    self.username = Some(text.to_string().clone());
                    USERNAME_UUID_MAP
                        .entry(text.clone().to_string())
                        .or_default()
                        .insert(self.id.clone());
                    self.state = SessionState::AwaitingRecipient;
                    ctx.text("Enter the username of the person you want to connect with:");
                }

                SessionState::AwaitingRecipient => {
                    let matches: Vec<_> = USERNAME_UUID_MAP
                        .get(&text.to_string())
                        .map(|set| {
                            set.iter()
                                .map(|entry| entry.clone())
                                .collect::<Vec<String>>()
                        })
                        .unwrap_or_else(Vec::new);

                    if matches.is_empty() {
                        ctx.text("No users found with that name.");
                    } else if matches.len() == 1 {
                        self.state = SessionState::Ready;
                        self.connected_to = Some(matches[0].clone().to_string());
                        self.send_message_to_uuid(&matches[0], ctx);
                    } else {
                        let list = matches
                            .iter()
                            .map(|uuid| format!("- {} (UUID: {})", text, uuid))
                            .collect::<Vec<_>>()
                            .join("\n");
                        ctx.text(format!(
                            "Multiple users found. Please choose a UUID:\n{}",
                            list
                        ));
                        self.state =
                            SessionState::AwaitingRecipientChoice(text.clone().to_string());
                    }
                }

                SessionState::AwaitingRecipientChoice(ref username) => {
                    let selected_uuid = text.clone();
                    if CONNECTED_CLIENTS.contains_key(&selected_uuid.to_string()) {
                        self.connected_to = Some(selected_uuid.clone().to_string());
                        self.state = SessionState::Ready;
                        self.send_message_to_uuid(&selected_uuid, ctx);
                    } else {
                        ctx.text("Invalid UUID. Please try again.");
                    }
                }

                SessionState::Ready => {
                    handle_chat_message(&text, self, ctx);
                }
            },
            Ok(ws::Message::Ping(msg)) => {
                println!("Ping received{}", std::str::from_utf8(&msg).unwrap());
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                println!("Received WebSocket Pong!");
            }
            Ok(ws::Message::Close(reason)) => {
                info!("WebSocket closing: {:?}", reason);
                ctx.stop();
            }
            _ => (),
        }
    }
}
impl WebSocketSession {
    fn send_message_to_uuid(&self, uuid: &str, ctx: &mut ws::WebsocketContext<Self>) {
        if let Some(sender) = CONNECTED_CLIENTS.get(uuid) {
            let msg = format!("Connected to {}", uuid);
            let _ = sender.send(self.id.clone());
            ctx.text(msg);
        } else {
            ctx.text("User UUID not found");
        }
    }
}

fn handle_chat_message(
    text: &str,
    session: &mut WebSocketSession,
    ctx: &mut ws::WebsocketContext<WebSocketSession>,
) {
    if let Some(target_uuid) = session.connected_to.as_ref() {
        if let Some(target_sender) = CONNECTED_CLIENTS.get(target_uuid) {
            let full_msg = format!(
                "{}: {}",
                session.username.as_deref().unwrap_or("anon"),
                text
            );
            let _ = target_sender.send(full_msg.clone());
            ctx.text(format!("Message sent to {}", target_uuid));
        } else {
            ctx.text("The recipient is no longer online.");
        }
    } else {
        ctx.text("You are not connected to anyone yet.");
    }
}

async fn ws_handler(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let (tx, rx) = flume::unbounded();
    let session_id = uuid::Uuid::new_v4().to_string();
    println!("WebSocket session uuid: {}", session_id);

    ws::start(
        WebSocketSession {
            id: session_id,
            username: None,
            state: SessionState::AwaitingUsername,
            connected_to: None,
            sender: tx,
            receiver: rx,
        },
        &req,
        stream,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    HttpServer::new(|| App::new().route("/ws", web::get().to(ws_handler)))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
