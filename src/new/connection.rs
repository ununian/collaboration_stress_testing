use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use yrs::Doc;

use crate::message::{AuthenticationMessage, IncomingMessage, MessageType, UpdateMessage};
use crate::peer::DocInfo;
use crate::util::to_hex_string;

pub async fn new_create_connection(
    url: &str,
) -> (
    mpsc::UnboundedSender<Message>,
    mpsc::UnboundedReceiver<Message>,
) {
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket 连接成功");
    let (mut write, mut read) = ws_stream.split();

    // 创建一个用于发送消息的通道
    let (ws_sender, mut ws_sender_rx) = mpsc::unbounded_channel::<Message>();

    let _ = tokio::spawn(async move {
        while let Some(msg) = ws_sender_rx.recv().await {
            println!("发送消息 {:?}", msg);
            write.send(msg).await.expect("Failed to send message");
        }
    });

    let (tx_ws_receiver, ws_receiver) = mpsc::unbounded_channel::<Message>();

    let _ = tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) if msg.is_binary() => {
                    match msg.clone() {
                        Message::Binary(bin) => {
                            println!("收到 WS 消息 \n{}\n -----", to_hex_string(&bin));
                        }
                        _ => {}
                    }
                    let _ = tx_ws_receiver.send(msg);
                }
                Err(e) => {
                    println!("WebSocket error: {:?}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    (ws_sender, ws_receiver)
}

struct YjsConnection {
    doc: Arc<AsyncMutex<Doc>>,
    info: DocInfo,
    ws_sender: mpsc::UnboundedSender<Message>,
    ws_receiver: mpsc::UnboundedReceiver<Message>,
}

pub async fn new_create_yjs_connection(info: &DocInfo) {
    let (ws_sender, mut ws_receiver) = new_create_connection(&info.url).await;

    let doc = Arc::new(AsyncMutex::new(Doc::new()));
    let mut conn = YjsConnection {
        doc: doc.clone(),
        info: info.clone(),
        ws_sender,
        ws_receiver,
    };

    let (yjs_message_tx, mut yjs_message_rx) = mpsc::unbounded_channel::<IncomingMessage>();

    let a = tokio::spawn(async move {
        while let Some(msg) = conn.ws_receiver.recv().await {
            match msg {
                Message::Binary(bin) => {
                    let msg = IncomingMessage::decode(bin);
                    println!("收到 Yjs 消息 {:?}", msg);
                    if !yjs_message_tx.is_closed() {
                        let _ = yjs_message_tx.send(msg);
                    }
                }
                _ => {
                    println!("未知消息类型");
                }
            }
        }
    });

    let auth_message = AuthenticationMessage {
        token: info.token.clone(),
        document_code: info.document_code.clone(),
    }
    .encode();

    conn.ws_sender.send(Message::Binary(auth_message)).unwrap();

    a.await.expect("Task panicked");
}
