use futures_util::{SinkExt, StreamExt};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::sync::{Mutex as AsyncMutex, RwLock};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use yrs::Doc;

use crate::client::DocInfo;
use crate::message::{
    AuthenticationMessage, IncomingMessage, MessageDecode, MessageType, UpdateMessage,
};

type ArcTx = Arc<AsyncMutex<mpsc::UnboundedSender<Message>>>;
// 定义回调函数的类型
type OnMessageCallback = Box<dyn Fn(Message, ArcTx) + Send + Sync>;
type OnSyncedCallback = Box<dyn Fn(Arc<AsyncMutex<Doc>>) + Send + Sync>;

// 构造函数创建 WebSocket 连接并返回一个 sender，用于向 WebSocket 发送消息
pub async fn create_connection(
    url: &str,
    on_message: OnMessageCallback,
) -> (ArcTx, JoinHandle<()>) {
    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket 连接成功");

    let (mut write, mut read) = ws_stream.split();

    // 创建一个用于发送消息的通道
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let tx_arc = Arc::new(AsyncMutex::new(tx));

    // 处理读取到的消息
    let tx_clone = tx_arc.clone();
    let handle = tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) if msg.is_binary() => {
                    println!("Received a message: {}", msg);
                    on_message(msg, tx_clone.clone()); // 调用回调函数
                }
                Err(e) => {
                    println!("WebSocket error: {:?}", e);
                }
                _ => {}
            }
        }
    });

    // 监听来自外部的消息并发送
    let _write_handle = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            println!("Sending message: {}", msg);
            write.send(msg).await.expect("Failed to send message");
        }
    });

    (tx_arc, handle)
}

pub async fn create_yjs_connection(
    doc: Arc<AsyncMutex<Doc>>,
    info: DocInfo,
    name: String,
    on_sync: OnSyncedCallback,
) {
    let document_code = info.document_code.clone();
    let on_name = name.clone();
    let on_doc = Arc::clone(&doc);

    let callback = Arc::new(on_sync);

    let (message_tx, mut message_rx) = mpsc::unbounded_channel();

    let message_tx_clone = message_tx.clone();
    let on_ws_message = move |msg: Message, tx: ArcTx| match msg {
        Message::Binary(bin) => {
            let msg = IncomingMessage::decode(bin);
            message_tx_clone.send(msg).unwrap();
        }
        _ => {
            println!("未知消息类型");
        }
    };

    let (tx, handle) = create_connection(&info.url, Box::new(on_ws_message)).await;

    {
        let info = info.clone();
        let auth_message = AuthenticationMessage {
            token: info.token.clone(),
            document_code: info.document_code.clone(),
        }
        .encode();

        tx.lock().await.send(Message::Binary(auth_message)).unwrap();
    }

    let mut subscripts = vec![];
    {
        let document_code = info.document_code.clone();
        println!("{} 开始监听更新", name);
        let doc = doc.clone();
        let tx = tx.clone();
        let doc = doc.lock().await;
        println!("{} 开始监听更新111", name);
        println!("监听doc {:p}", &doc);
        subscripts.push(
            doc.observe_update_v1(move |_, event| {
                println!("{} 观察到更新", name);
                let msg = UpdateMessage {
                    update: event.update.clone(),
                    document_code: document_code.clone(),
                }
                .encode();
                let tx = tx.clone();
                tokio::spawn(async move {
                    let tx = tx.lock().await;
                    if let Err(e) = tx.send(Message::Binary(msg)) {
                        println!("发送更新消息失败: {:?}", e);
                    };
                });
            })
            .unwrap(),
        );
    }

    let doc_arc = Arc::clone(&doc);
    let name = on_name.clone();
    let document_code = document_code.clone();
    let callback = callback.clone();

    while let Some(msg) = message_rx.recv().await {
        let doc = doc_arc.lock().await;
        let response = msg.message_type.handle_message(&document_code, &doc, &name);

        if let Some(response) = response {
            let tx = tx.lock().await;
            if tx.send(Message::Binary(response)).is_err() {
                println!("Failed to send message");
            }
        }

        match msg.message_type {
            MessageType::Sync(crate::message::SyncStep::Two(_)) => {
                println!("sync step 2 doc {:p}", &doc);
                doc.observe_update_v1(|_, _| {
                    println!("观察到更新22222");
                })
                .unwrap();
                callback(Arc::clone(&doc_arc));
            }
            _ => {}
        }
    }
}
