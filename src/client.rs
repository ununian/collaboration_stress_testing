use crate::message::{
    AuthenticationMessage, IncomingMessage, MessageDecode, MessageEncode, MessageType, SyncStep,
    UpdateMessage,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    any::Any,
    borrow::{Borrow, BorrowMut},
    sync::{Arc, Mutex},
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
    pub doc: Arc<Doc>,
}

impl DocClient {
    pub fn new(info: DocInfo) -> Self {
        let doc = Doc::new();

        DocClient {
            doc: Arc::new(doc),
            info,
        }
    }

    pub async fn init<F>(&self, on_synced: F)
    where
        F: Fn(Arc<Doc>) + 'static + Send + Sync,
    {
        let (ws_stream, _) = connect_async(&self.info.url)
            .await
            .expect("创建 WS 连接失败");

        let (mut write, mut read) = ws_stream.split();

        let (send, mut recv) = channel(1024);

        tokio::spawn(async move {
            while let Some(msg) = recv.recv().await {
                let _ = write.send(Message::Binary(msg)).await;
            }
        });

        let info = self.info.clone();

        self.doc.get_or_insert_map("blocks").observe_deep(|_, e| {
            println!("观察到变化");
            e.iter().for_each(|e| {
                println!("{:?}", e.path());
            });
        });

        let document_code = info.document_code.clone();
        let update_sender = send.clone();
        let doc_subscription = {
            self.doc
                .observe_update_v1(move |_, event| {
                    // https://github.com/y-crdt/y-sync/blob/56958e83acfd1f3c09f5dd67cf23c9c72f000707/src/net/broadcast.rs#L47-L52
                    let msg = UpdateMessage {
                        update: event.update.clone(),
                        document_code: document_code.clone(),
                    };

                    let update_sender = update_sender.clone();
                    tokio::spawn(async move {
                        update_sender.send(msg.encode()).await.unwrap();
                    });
                })
                .unwrap()
        };

        let doc = self.doc.clone();
        let send = send.clone();
        tokio::spawn(async move {
            let auth_message = AuthenticationMessage {
                token: (info.token).clone(),
                document_code: info.document_code.clone(),
            };

            send.send(auth_message.encode()).await.unwrap();

            while let Some(Ok(msg)) = read.next().await {
                match msg {
                    Message::Binary(bin) => {
                        let msg = IncomingMessage::decode(&bin);
                        let response = msg.message_type.handle_message(&info, &doc);

                        if let Some(response) = response {
                            send.send(response).await.unwrap();
                        }

                        match msg.message_type {
                            MessageType::Sync(step) => match step {
                                SyncStep::Two(_) => {
                                    on_synced(doc.clone());
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    }
                    _ => {
                        println!("未知消息类型");
                    }
                }
            }
        })
        .await
        .unwrap();
    }

    // pub fn listen(&self) {
    //     let doc = self.doc.clone();
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
