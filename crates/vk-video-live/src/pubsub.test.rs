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
