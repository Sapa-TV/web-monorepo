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
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    const CHAT_CHANNEL: &str = "channel-chat:4242";
    const PUSH_1: &str = r#"{"push":{"channel":"channel-chat:4242","pub":{"data":{"type":"message","data":{"id":1,"createdAt":10,"author":{"id":2,"nick":"a"},"data":[{"type":"text","content":"[\"hi\",\"\"]"}]}}}}}"#;
    const PUSH_2: &str = r#"{"push":{"channel":"channel-chat:4242","pub":{"data":{"type":"message","data":{"id":2,"createdAt":11,"author":{"id":2,"nick":"a"},"data":[{"type":"text","content":"[\"yo\",\"\"]"}]}}}}}"#;

    #[tokio::test]
    async fn connect_subscribe_receive_with_ping_pong() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            let connect_frame = ws.next().await.unwrap().unwrap();
            let connect_text = connect_frame.to_string();
            assert!(connect_text.contains(r#""token":"tok""#));
            assert!(connect_text.contains(r#""name":"sapa-tv""#));
            ws.send(Message::text(
                r#"{"id":1,"connect":{"client":"c","ping":25}}"#,
            ))
            .await
            .unwrap();

            let subscribe_frame = ws.next().await.unwrap().unwrap();
            assert!(subscribe_frame.to_string().contains(CHAT_CHANNEL));
            ws.send(Message::text(r#"{"id":2,"subscribe":{}}"#))
                .await
                .unwrap();

            ws.send(Message::text(PUSH_1)).await.unwrap();
            ws.send(Message::text(r#"{"ping":7}"#)).await.unwrap();
            ws.send(Message::text(PUSH_2)).await.unwrap();

            let pong = ws.next().await.unwrap().unwrap();
            assert_eq!(pong.to_string(), r#"{"pong":7}"#);

            ws.close(None).await.unwrap();
        });

        let url = format!("ws://{addr}/connection/websocket?format=json&cf_protocol_version=v2");
        let mut client = PubSub::connect(&url, "tok").await.unwrap();
        client.subscribe(CHAT_CHANNEL).await.unwrap();

        let first = client.next_push().await.unwrap();
        assert!(first.contains(r#""id":1"#));
        let second = client.next_push().await.unwrap();
        assert!(second.contains(r#""id":2"#));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn subscribe_error_is_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            let _ = ws.next().await.unwrap().unwrap();
            ws.send(Message::text(
                r#"{"id":1,"connect":{"client":"c","ping":25}}"#,
            ))
            .await
            .unwrap();
            let _ = ws.next().await.unwrap().unwrap();
            ws.send(Message::text(
                r#"{"id":2,"subscribe":{"error":{"code":103,"message":"insufficient scopes"}}}"#,
            ))
            .await
            .unwrap();
            ws.close(None).await.unwrap();
        });

        let url = format!("ws://{addr}/connection/websocket");
        let mut client = PubSub::connect(&url, "tok").await.unwrap();
        let res = client.subscribe(CHAT_CHANNEL).await;
        assert!(matches!(res, Err(Error::Protocol(m)) if m.contains("insufficient scopes")));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn connect_rejection_is_reported() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            let _ = ws.next().await.unwrap().unwrap();
            ws.send(Message::text(
                r#"{"id":1,"error":{"code":101,"message":"invalid token"}}"#,
            ))
            .await
            .unwrap();
            ws.close(None).await.unwrap();
        });

        let url = format!("ws://{addr}/connection/websocket");
        let res = PubSub::connect(&url, "bad").await;
        assert!(matches!(res, Err(Error::Protocol(m)) if m.contains("invalid token")));

        server.await.unwrap();
    }

    #[tokio::test]
    async fn closed_connection_is_protocol_error() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();
            let _ = ws.next().await.unwrap().unwrap();
            ws.send(Message::text(
                r#"{"id":1,"connect":{"client":"c","ping":25}}"#,
            ))
            .await
            .unwrap();
            ws.close(None).await.unwrap();
        });

        let url = format!("ws://{addr}/connection/websocket");
        let mut client = PubSub::connect(&url, "tok").await.unwrap();
        let res = client.next_push().await;
        assert!(matches!(res, Err(Error::Protocol(m)) if m.contains("closed")));

        server.await.unwrap();
    }
}
