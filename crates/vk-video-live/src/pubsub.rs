use std::collections::VecDeque;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::error::{Error, Result};

pub const PUBSUB_URL: &str =
    "wss://pubsub.live.vkvideo.ru/connection/websocket?format=json&cf_protocol_version=v2";

pub struct PubSub {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    next_id: u64,
    pending: VecDeque<String>,
}

impl PubSub {
    pub async fn connect(url: &str, token: &str) -> Result<Self> {
        let (ws, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| Error::Protocol(format!("ws connect: {e}")))?;
        let mut client = Self {
            ws,
            next_id: 1,
            pending: VecDeque::new(),
        };
        let id = client
            .send_frame(json!({ "connect": { "token": token, "name": "sapa-tv" } }))
            .await?;
        let reply = client.reply(id).await?;
        if let Some(error) = reply.get("error") {
            return Err(Error::Protocol(format!("connect rejected: {error}")));
        }
        if reply.get("connect").is_none() {
            return Err(Error::Protocol("connect reply missing".to_string()));
        }
        Ok(client)
    }

    pub async fn subscribe(&mut self, channel: &str) -> Result<()> {
        let id = self
            .send_frame(json!({ "subscribe": { "channel": channel } }))
            .await?;
        let reply = self.reply(id).await?;
        if let Some(error) = reply
            .get("error")
            .or_else(|| reply.pointer("/subscribe/error"))
        {
            return Err(Error::Protocol(format!("subscribe rejected: {error}")));
        }
        Ok(())
    }

    pub async fn next_push(&mut self) -> Result<String> {
        if let Some(frame) = self.pending.pop_front() {
            return Ok(frame);
        }
        loop {
            let text = self.read_text().await?;
            let value = parse_frame(&text)?;
            if let Some(ping) = value.get("ping") {
                self.ws
                    .send(Message::text(json!({ "pong": ping }).to_string()))
                    .await
                    .map_err(|e| Error::Protocol(format!("ws send: {e}")))?;
                continue;
            }
            if value.get("push").is_some() {
                return Ok(text);
            }
        }
    }

    pub async fn close(&mut self) {
        self.ws.close(None).await.ok();
    }

    async fn send_frame(&mut self, mut frame: Value) -> Result<u64> {
        let id = self.next_id;
        self.next_id += 1;
        frame["id"] = json!(id);
        self.ws
            .send(Message::text(frame.to_string()))
            .await
            .map_err(|e| Error::Protocol(format!("ws send: {e}")))?;
        Ok(id)
    }

    async fn reply(&mut self, id: u64) -> Result<Value> {
        loop {
            let text = self.read_text().await?;
            let value = parse_frame(&text)?;
            if value.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(value);
            }
            if let Some(ping) = value.get("ping") {
                self.ws
                    .send(Message::text(json!({ "pong": ping }).to_string()))
                    .await
                    .map_err(|e| Error::Protocol(format!("ws send: {e}")))?;
                continue;
            }
            if value.get("push").is_some() {
                self.pending.push_back(text);
            }
        }
    }

    async fn read_text(&mut self) -> Result<String> {
        loop {
            let msg = self
                .ws
                .next()
                .await
                .ok_or_else(|| Error::Protocol("websocket closed".to_string()))?
                .map_err(|e| Error::Protocol(format!("websocket: {e}")))?;
            match msg {
                Message::Text(text) => return Ok(text.to_string()),
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => continue,
                Message::Close(_) => return Err(Error::Protocol("websocket closed".to_string())),
                _ => continue,
            }
        }
    }
}

fn parse_frame(text: &str) -> Result<Value> {
    serde_json::from_str(text).map_err(|e| Error::Protocol(format!("ws frame: {e}")))
}

#[cfg(test)]
#[path = "pubsub.test.rs"]
mod tests;
