use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::message::{AuthenticationMessage, IncomingMessage, MessageType};
use crate::peer::DocInfo;

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
            // println!("发送消息 {:?}", msg);
            write.send(msg).await.expect("Failed to send message");
        }
    });

    let (tx_ws_receiver, ws_receiver) = mpsc::unbounded_channel::<Message>();

    let _ = tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) if msg.is_binary() => {
                    // match msg.clone() {
                    //     Message::Binary(bin) => {
                    //         // println!("收到 WS 消息 \n{}\n -----", to_hex_string(&bin));
                    //     }
                    //     _ => {}
                    // }
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

pub struct YjsConnection {
    pub info: DocInfo,
    pub ws_sender: mpsc::UnboundedSender<Message>,
    pub yjs_rx: UnboundedReceiver<IncomingMessage>,
}

impl DocInfo {
    pub async fn connection(&self) -> YjsConnection {
        let (ws_sender, mut ws_receiver) = new_create_connection(&self.url).await;

        let (yjs_message_tx, yjs_message_rx) = mpsc::unbounded_channel::<IncomingMessage>();
        let (emit_auth, mut on_auth) = mpsc::channel::<bool>(1);

        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.recv().await {
                match msg {
                    Message::Binary(bin) => {
                        let msg = IncomingMessage::decode(bin);
                        // println!(
                        //     "收到 Yjs 消息 [{} | {:?}]",
                        //     msg.document_code, msg.message_type
                        // );

                        if !yjs_message_tx.is_closed() {
                            match msg.message_type {
                                MessageType::Auth(result) => {
                                    if emit_auth.is_closed() {
                                        continue;
                                    }
                                    if let Ok(scope) = result {
                                        if scope == "read-write" {
                                            let _ = emit_auth.send(true).await;
                                            continue;
                                        }
                                    }
                                    println!("认证失败");
                                    let _ = emit_auth.send(false).await;
                                }
                                _ => {
                                    let _ = yjs_message_tx.send(msg);
                                }
                            }
                        } else {
                            println!("通道已关闭");
                            break;
                        }
                    }
                    _ => {
                        println!("未知消息类型");
                    }
                }
            }
        });

        let auth_message = AuthenticationMessage {
            token: self.token.clone(),
            document_code: self.document_code.clone(),
        }
        .encode();

        let conn = YjsConnection {
            info: self.clone(),
            ws_sender,
            yjs_rx: yjs_message_rx,
        };

        conn.ws_sender.send(Message::Binary(auth_message)).unwrap();

        let auth_result = on_auth.recv().await;
        println!("auth_result: {:?}", auth_result);

        conn
    }
}
