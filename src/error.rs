use actix::prelude::*;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub enum WebSocketError {
    ClientAlreadyConnected,
    MessageParsingError,
    ProtocolError(ws::ProtocolError),
    ActixError(actix_web::Error),
}

impl std::error::Error for WebSocketError {}

impl std::fmt::Display for WebSocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            WebSocketError::ClientAlreadyConnected => write!(f, "Client already connected"),
            WebSocketError::MessageParsingError => write!(f, "Message parsing error"),
            WebSocketError::ProtocolError(err) => write!(f, "Protocol error: {}", err),
            WebSocketError::ActixError(err) => write!(f, "Actix error: {}", err),
        }
    }
}

impl From<ws::ProtocolError> for WebSocketError {
    fn from(err: ws::ProtocolError) -> WebSocketError {
        WebSocketError::ProtocolError(err)
    }
}

impl From<actix_web::Error> for WebSocketError {
    fn from(err: actix_web::Error) -> WebSocketError {
        WebSocketError::ActixError(err)
    }
}
