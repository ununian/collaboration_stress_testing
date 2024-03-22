mod client;
mod connection;
mod id;
mod message;
mod model;
mod query;
mod thread;
mod util;

use connection::create_yjs_connection;
use query::query::DocQuery;
use std::borrow::Borrow;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use futures_util::{never, SinkExt, StreamExt};
use nanoid::nanoid;
use tokio::time::{interval, sleep_until, Instant};
use tokio::{time::sleep, time::Duration};

use client::{DocClient, DocInfo};
use tokio_tungstenite::connect_async;
use yrs::types::ToJson;
use yrs::{
    Any, ArrayPrelim, DeepObservable, Doc, GetString, Map, MapPrelim, MapRef, Observable, TextRef,
};
use yrs::{Text, TextPrelim, Transact, Value};

use crate::connection::create_connection;
use tokio::sync::{Mutex as AsyncMutex, RwLock};
type WsMessage = tokio_tungstenite::tungstenite::Message;
type SyncMessage = yrs::sync::Message;

struct Peer {
    id: String,
}

#[tokio::main]
async fn main() {
    let url = "ws://10.5.23.192:8896/aio";

    let info = DocInfo {
        token: "i7Hm6ULQsDNIWSVJT2IKV9Bpl-o".to_string(),
        document_code: "Page::50304457394944::DEFAULT_PAGE".to_string(),
        url: "ws://10.5.23.192:8896/aio".to_string(),
    };

    let id = Arc::new(nanoid!());

    let mut handles = Vec::new();

    {
        let count = Arc::new(Mutex::new(1));

        let doc = Arc::new(AsyncMutex::new(Doc::new()));
        let id_clone = id.clone();
        let handle1 = tokio::spawn(create_yjs_connection(
            Arc::clone(&doc),
            info.clone(),
            String::from("Thread_1"),
            Box::new(move |doc| {
                let doc = Arc::clone(&doc);
                let id = id_clone.clone();
                tokio::spawn(async move {
                    println!("同步完成");
                    let doc: tokio::sync::MutexGuard<'_, Doc> = doc.lock().await;
                    println!("同步doc {:p}", &doc);
                    let block_map = doc.get_or_insert_map("blocks");

                    {
                        let mut txn = doc.transact_mut();
                        println!("blocks {}", block_map.len(&txn));
                        let blocks = block_map
                            .iter(&txn)
                            .map(|(_, block)| match block {
                                Value::YMap(block) => {
                                    return Some(block);
                                }
                                _ => None,
                            })
                            .filter(|block| block.is_some())
                            .map(|block| block.unwrap())
                            .collect::<Vec<_>>();

                        let note_block =
                            blocks
                                .iter()
                                .find(|block| match block.get(&txn, "sys:flavour") {
                                    Some(Value::Any(Any::String(flavor))) => {
                                        println!(
                                            "flavor: {},{}",
                                            flavor,
                                            flavor == "wq:note".into()
                                        );
                                        return flavor == "wq:note".into();
                                    }
                                    _ => false,
                                });

                        if note_block.is_none() {
                            println!("没有找到根节点");
                            return;
                        }

                        println!("11111");
                        let map: MapRef = block_map.insert(
                            &mut txn,
                            id.clone().as_str(),
                            MapPrelim::from([
                                (
                                    "sys:id".to_string(),
                                    Any::String(id.clone().as_str().into()),
                                ),
                                ("sys:version".to_string(), Any::Number(1.into())),
                                (
                                    "sys:flavour".to_string(),
                                    Any::String("wq:paragraph".into()),
                                ),
                            ]),
                        );
                        println!("22222");
                        map.insert(&mut txn, "prop:text", TextPrelim::new("ping_0"));
                        map.insert(&mut txn, "sys:children", ArrayPrelim::default());

                        println!("33333");
                    }
                });
            }),
        ));

        let id_clone = id.clone();
        let count_clone = count.clone();
        let worker_doc = Arc::clone(&doc);

        // tokio::spawn(async move {
        //     let doc = worker_doc.lock().await;
        //     let blocks = doc.get_or_insert_map("blocks");
        //     let id = id_clone.clone();

        //     blocks.observe(move |txn, event| {
        //         let block = event.target();
        //         let id = block.get(txn, "sys:id");
        //         let id = match id {
        //             Some(Value::Any(Any::String(id))) => id,
        //             _ => panic!("没有找到ID"),
        //         };

        //         let prop = block.get(txn, "prop:text");
        //         let prop = match prop {
        //             Some(Value::Any(Any::String(prop))) => prop,
        //             _ => panic!("没有找到Text"),
        //         };

        //         let count = count_clone.lock().unwrap();
        //         if prop.ends_with(format!("pong_{} ", *count).as_str()) {}
        //     });

        //     // let block = while let Some(msg) = blocks.get(&txn, id.clone().as_str()) {
        //     //     match msg {
        //     //         Value::YMap(block) => {
        //     //             sleep(Duration::from_secs(1)).await;
        //     //             continue;
        //     //         }
        //     //         _ => {
        //     //             sleep(Duration::from_secs(1)).await;
        //     //             continue;
        //     //         }
        //     //     }
        //     // };
        // });

        // let client = Arc::new(RwLock::new(DocClient::new("thread_1", &info.clone())));
        // tokio::spawn(DocClient::auto_update(Arc::clone(&client)));

        // let handle1 = tokio::spawn(async move {
        //     let client = Arc::new(RwLock::new(DocClient::new("thread_1", &info.clone())));
        //     let (ws_stream, _) = connect_async(url).await.expect("创建 WS 连接失败");

        //     let (ws_send, mut ws_recv) = channel::<WsMessage>(1024);
        //     let (message_send, mut message_recv) = channel::<WsMessage>(1024);

        //     let (mut write, mut read) = ws_stream.split();

        //     {
        //         tokio::spawn(async move {
        //             while let Some(msg) = ws_recv.recv().await {
        //                 write.send(msg).await.unwrap();
        //             }
        //         });
        //     }

        //     {
        //         let message_send = message_send.clone();
        //         tokio::spawn(async move {
        //             while let Some(Ok(msg)) = read.next().await {
        //                 message_send.send(msg).await;
        //             }
        //         });
        //     }

        // DocClient::auto_update(client).await;

        // client
        //     .init(
        //         move |doc| {
        //             println!("同步完成111");
        //             {
        //                 let query = DocQuery::new(Arc::clone(&doc));

        //                 println!("id: {}", id.clone());
        //                 let prop = query
        //                     .blocks()
        //                     .add_block(id.clone().as_str(), "wq:paragraph", |mut txn, prop| {
        //                         prop.insert(&mut txn, "prop:text", TextPrelim::new(""));
        //                     })
        //                     .prop("prop:text")
        //                     .get();

        //                 let mut text = match prop {
        //                     Some(Value::YText(text)) => text,
        //                     _ => panic!("没有找到Text"),
        //                 };

        //                 println!("11111");
        //                 {
        //                     let send = send.clone();
        //                     tokio::spawn(
        //                         async move { send.send(format!("ping_{} ", 0)).await },
        //                     );
        //                 }
        //                 let send = send.clone();
        //                 let count = count.clone();
        //                 text.observe(move |txn, event| {
        //                     let text = event.target().get_string(txn);
        //                     let mut count = count.lock().unwrap();
        //                     if text.ends_with(format!("pong_{} ", *count).as_str()) {
        //                         *count += 1;
        //                         let count = count.clone();
        //                         let _ = send.send(format!("ping_{} ", count));
        //                     }
        //                 });
        //             }
        //         },
        //         String::from("Thread_1"),
        //     )
        //     .await;
        // });

        handles.push(handle1);
    }

    // {
    //     let info = info.clone();
    //     let id = id.clone();
    //     let count = count.clone();
    //     let client = Arc::new(Mutex::new(DocClient::new(&info.clone())));
    //     let (send, mut recv) = channel::<String>(1024);

    //     {
    //         let client = Arc::clone(&client);
    //         handles.push(tokio::spawn(async move {
    //             client
    //                 .lock()
    //                 .unwrap()
    //                 .init(
    //                     move |doc| {
    //                         println!("同步完成222");
    //                     },
    //                     String::from("Thread_2"),
    //                 )
    //                 .await;
    //         }));
    //     }

    //     {
    //         let client = Arc::clone(&client);
    //         handles.push(tokio::spawn(async move {
    //             loop {
    //                 let doc = client.lock().unwrap().doc;
    //                 println!("333333312312312");
    //                 let query = DocQuery::new(Arc::clone(&doc));
    //                 let block = query.blocks().id(id.clone().as_str());
    //                 let prop = block.and_then(|block| block.prop("prop:text").get());
    //                 let text = match prop {
    //                     Some(Value::YText(text)) => Some(text),
    //                     _ => None,
    //                 };
    //                 if text.is_none() {
    //                     sleep(Duration::from_secs(1)).await;
    //                     continue;
    //                 }
    //                 let text = text.unwrap();

    //                 text.observe(|txn, event| {
    //                     let text = event.target().get_string(txn);
    //                     println!("text: {}", text);
    //                 });
    //                 break;
    //             }
    //         }));
    //     }

    // {
    //     let client = Arc::clone(&client);
    //     handles.push(tokio::spawn(async move {
    //         while let Some(msg) = recv.recv().await {
    //             println!("333333312312312");
    //             let query = DocQuery::new(Arc::clone(&doc));
    //             let block = query.blocks().id(id.clone().as_str());
    //             let prop = block.and_then(|block| block.prop("prop:text").get());
    //             let text = match prop {
    //                 Some(Value::YText(text)) => text,
    //                 _ => panic!("没有找到Text"),
    //             };
    //             let mut txn = doc.transact_mut();
    //             text.push(&mut txn, msg.as_str());
    //         }
    //     }));
    // }
    // }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for ctrl+C");

    sleep(Duration::from_secs(1)).await;

    // let mut handles = Vec::new(); // 用于存放所有任务的句柄

    // for i in 0..50 {
    //     let mut info = info.clone();
    //     info.content = format!("thread{} ", i);
    //     let handle = tokio::spawn(async move {
    //         sleep(Duration::from_secs_f64((i as f64) * 0.5)).await;
    //         start_client(&info).await;
    //     });
    //     handles.push(handle);
    // }

    // // 等待所有异步任务完成
    // for handle in handles {
    //     handle.await.expect("Task panicked");
    // }
}

// async fn start_client(info: &DocInfo) {
//     let doc = Doc::new();
//     let mut client = Client { doc };

//     // WebSocket 服务地址

//     // 连接到 WebSocket 服务
//     let (ws_stream, _) = connect_async(&info.url).await.expect("Failed to connect");
//     println!("WebSocket client connected");
//     // 分离发送与接收处理器
//     let (mut write, mut read) = ws_stream.split();

//     let auth_message = AuthenticationMessage {
//         token: (info.token).clone(),
//         document_code: info.document_code.clone(),
//     };

//     write
//         .send(Message::Binary(auth_message.encode()))
//         .await
//         .expect("Failed to send message");

//     loop {
//         // 等待并接收消息
//         match read.next().await {
//             Some(Ok(message)) => {
//                 match message {
//                     // 仅处理二进制消息
//                     Message::Binary(bin) => {
//                         println!("{}", to_hex_string(&bin));
//                         let msg = IncomingMessage::decode(&bin);
//                         // println!(
//                         //     "{:?}, {:?} {} {}",
//                         //     msg.document_code,
//                         //     msg.message_type,
//                         //     msg.cur.buf.len(),
//                         //     msg.cur.next
//                         // );

//                         match next_step(&msg.message_type, &info, &mut client) {
//                             Some(buf) => {
//                                 if buf.len() == 0 {
//                                     break;
//                                 }
//                                 let response_message = Message::Binary(buf);
//                                 match write.send(response_message).await {
//                                     Ok(_) => println!("Sent a response binary message"),
//                                     Err(e) => {
//                                         eprintln!("Failed to send message: {}", e);
//                                         break; // 出错时退出循环
//                                     }
//                                 }
//                             }
//                             None => {
//                                 continue;
//                             }
//                         }
//                         // 响应发送二进制消息
//                         // let response_message = Message::Binary(vec![6, 7, 8, 9, 0]); // 举例，你应根据需要修改此处
//                         // match write.send(response_message).await {
//                         //     Ok(_) => println!("Sent a response binary message"),
//                         //     Err(e) => {
//                         //         eprintln!("Failed to send message: {}", e);
//                         //         break; // 出错时退出循环
//                         //     }
//                         // }
//                     }
//                     _ => println!("Received a non-binary message"),
//                 }
//             }
//             Some(Err(e)) => {
//                 eprintln!("Error during receiving message: {}", e);
//                 break; // 出错时退出循环
//             }
//             None => break, // 无更多消息时退出循环
//         }
//     }

//     let mut interval_timer = interval(Duration::from_secs_f64(0.1));
//     let mut count = 0;
//     loop {
//         interval_timer.tick().await; // 等待下一个间隔
//                                      // 准备你要发送的二进制消息
//         count += 1;
//         if count > 100 {
//             break;
//         }
//         let buf = insert_content_message(&info, &mut client).unwrap();

//         let response_message = Message::Binary(buf);
//         match write.send(response_message).await {
//             Ok(_) => println!("Sent a response binary message"),
//             Err(e) => {
//                 eprintln!("Failed to send message: {}", e);
//                 return;
//             }
//         }
//     }

//     // 给服务器一些时间来回应后再关闭连接
//     sleep(Duration::from_secs(1)).await;
// }
