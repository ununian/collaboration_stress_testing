use crate::message::{
    AuthenticationMessage, IncomingMessage, MessageDecode, MessageEncode, MessageType, SyncStep,
    UpdateMessage,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::mpsc::channel;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};
use yrs::{
    encoding::write::Write,
    updates::{decoder::Decode, encoder::EncoderV1},
    DeepObservable, Doc, Observable, Transact, Update,
};

#[derive(Debug, Clone)]
pub struct DocInfo {
    pub token: String,
    pub document_code: String,
    pub url: String,
}

pub struct DocClient {
    pub info: DocInfo,
    pub name: String,
    pub doc: Doc,
}

impl DocClient {
    pub fn new(name: &str, info: &DocInfo) -> Self {
        let doc = Doc::new();

        DocClient {
            doc: doc,
            name: name.to_string(),
            info: info.clone(),
        }
    }

    pub fn doc(&self) -> &Doc {
        &self.doc
    }

    /// Returns a read-write reference to an underlying [Doc].
    pub fn doc_mut(&mut self) -> &mut Doc {
        &mut self.doc
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn auto_update(client: Arc<RwLock<Self>>) {
        let r_client = client.read().unwrap();
        let (ws_send, mut ws_recv) = channel(1024);
        let (message_send, mut message_recv) = channel(1024);

        {
            let (ws_stream, _) = connect_async(&r_client.info.url)
                .await
                .expect("创建 WS 连接失败");

            let (mut write, mut read) = ws_stream.split();

            tokio::spawn(async move {
                while let Some(msg) = ws_recv.recv().await {
                    write.send(msg).await.unwrap();
                }
            });

            let message_send = message_send.clone();
            tokio::spawn(async move {
                while let Some(Ok(msg)) = read.next().await {
                    message_send.send(msg).await;
                }
            });
        }

        {
            let auth_message = AuthenticationMessage {
                token: (&r_client.info.token).clone(),
                document_code: (&r_client.info.document_code).clone(),
            }
            .encode();

            if let Err(e) = message_send.try_send(Message::Binary(auth_message)) {
                println!("发送认证消息失败: {:?}", e);
            }
        }

        {
            let name = r_client.name.clone();
            &r_client
                .doc()
                .get_or_insert_map("blocks")
                .observe_deep(move |_, e| {
                    println!("{} 观察到 blocks 变化", name);
                    e.iter().for_each(|e| {
                        println!("{:?}", e.path());
                    });
                });
        }

        {
            println!("{} 开始监听更新", &r_client.name());
            let ws_send = ws_send.clone();
            let name = r_client.name.clone();
            let document_code = r_client.info.document_code.clone();
            let _ = &r_client
                .doc()
                .observe_update_v1(move |_, event| {
                    println!("{} 观察到更新", name);
                    let msg = UpdateMessage {
                        update: event.update.clone(),
                        document_code: document_code.clone(),
                    }
                    .encode();

                    if let Err(e) = ws_send.try_send(Message::Binary(msg)) {
                        println!("发送更新消息失败: {:?}", e);
                    }
                })
                .unwrap();
        }

        // while let Some(msg) = message_recv.recv().await {
        //     match msg {
        //         Message::Binary(bin) => {
        //             let msg = IncomingMessage::decode(&bin);
        //             let response = msg.message_type.handle_message(
        //                 &r_client.info,
        //                 r_client.doc(),
        //                 r_client.name(),
        //             );

        //             if let Some(response) = response {
        //                 ws_send.send(Message::Binary(response)).await.unwrap();
        //             }
        //         }
        //         _ => {
        //             println!("未知消息类型");
        //         }
        //     }
        // }

        // tokio::spawn(async move {
        //     send.send(auth_message).await.unwrap();

        //     while let Some(Ok(msg)) = read.next().await {
        //         message_send.send(msg).await;
        // match msg {
        //     Message::Binary(bin) => {

        // let msg = IncomingMessage::decode(&bin);
        // let response =
        //     msg.message_type
        //         .handle_message(&info, r_client.doc(), r_client.name());

        // if let Some(response) = response {
        //     send.send(response).await.unwrap();
        // }

        // match msg.message_type {
        //     MessageType::Sync(step) => match step {
        //         SyncStep::Two(_) => {}
        //         _ => {}
        //     },
        //     _ => {}
        // }
        //     }
        //     _ => {
        //         println!("未知消息类型");
        //     }
        // }
        //     }
        // })
        // .await;
    }

    // pub fn listen(&self) {
    //     let doc = self.Arc::clone(&doc)();
    //     let client = self.client.clone();
    //     tokio::spawn(async move {
    //         let (mut write, mut read) = client.split();
    //         while let Some(Ok(msg)) = read.next().await {
    //             let msg = msg.into_text().unwrap();
    //             let update =
    //                 yrs::Update::decode(&mut yrs::encoding::read::Cursor::new(msg.as_bytes()));
    //             doc.transact(update);
    //         }
    //     });
    // }
}
