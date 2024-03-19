mod client;
mod id;
mod message;
mod model;
mod query;
mod thread;
mod util;

use std::borrow::Borrow;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use nanoid::nanoid;
use tokio::sync::mpsc::channel;
use tokio::time::interval;
use tokio::{time::sleep, time::Duration};

use client::{DocClient, DocInfo};
use yrs::types::ToJson;
use yrs::{DeepObservable, GetString, Map, Observable};
use yrs::{Text, TextPrelim, Transact, Value};

use crate::query::query::DocQuery;

struct Peer {
    id: String,
}

#[tokio::main]
async fn main() {
    let info = Arc::new(DocInfo {
        token: "167sD7a3eiDL3DZ4zjOrjzlGQ2g".to_string(),
        document_code: "Page::50305026731776::DEFAULT_PAGE".to_string(),
        url: "ws://10.5.23.192:8896/aio".to_string(),
    });

    let id = Arc::new(nanoid!());

    let info1 = info.clone();
    let id1 = id.clone();
    let handle1 = tokio::spawn(async move {
        let client = DocClient::new(&info1);
        client
            .init(move |doc| {
                println!("同步完成111");
                {
                    let query = DocQuery::new(doc.clone());

                    let prop = query
                        .blocks()
                        .add_block(id1.as_str(), "wq:paragraph", |mut txn, prop| {
                            prop.insert(&mut txn, "prop:text", TextPrelim::new(""));
                        })
                        .prop("prop:text")
                        .get();

                    let mut text = match prop {
                        Some(Value::YText(text)) => text,
                        _ => panic!("没有找到Text"),
                    };

                    let count = Arc::new(Mutex::new(0));

                    let (send, mut recv) = channel::<String>(1024);
                    let id1 = id1.clone();
                    tokio::spawn(async move {
                        while let Some(msg) = recv.recv().await {
                            let query = DocQuery::new(doc.clone());
                            let block = query.blocks().id(id1.as_str());
                            let prop = block.and_then(|block| block.prop("prop:text").get());
                            let text = match prop {
                                Some(Value::YText(text)) => text,
                                _ => panic!("没有找到Text"),
                            };
                            let mut txn = doc.transact_mut();
                            text.push(&mut txn, msg.as_str());
                        }
                    });

                    let send2 = send.clone();
                    text.observe(move |txn, event| {
                        let text = event.target().get_string(txn);
                        let mut count = count.lock().unwrap();
                        if text.ends_with(format!("pong_{} ", *count).as_str()) {
                            *count += 1;
                            let count = count.clone();
                            let send = send2.clone();
                            tokio::spawn(async move {
                                let _ = send.send(format!("ping_{} ", count)).await;
                            });
                        }
                    });

                    let first_sender = send.clone();
                    tokio::spawn(async move {
                        let _ = first_sender.send("ping_0 ".to_string()).await;
                    });
                }
            })
            .await;
    });

    let info2 = info.clone();
    let id2 = id.clone();
    let handle2 = tokio::spawn(async move {
        let client = DocClient::new(&info2.clone());
        client
            .init(move |_| {
                println!("同步完成222");
            })
            .await;
        let doc = client.doc.clone();
        let mut blocks = doc.get_or_insert_map("blocks");
        let id2 = id2.clone();
        let doc = doc.clone();
        let is_listener = Arc::new(Mutex::new(false));
        {
            let txn = doc.transact();
            println!("blocks: {:?}", blocks.get(&txn, id2.as_str()));
        }
        blocks.observe_deep(move |_, _| {
            println!("监听到Block变化111");

            if *is_listener.lock().unwrap() {
                return;
            }

            let query = DocQuery::new(doc.clone());
            let block = query.blocks().id(id2.as_str());
            if block.is_none() {
                return;
            }

            println!("监听到Block变化222");

            *is_listener.lock().unwrap() = true;

            let prop = block.unwrap().prop("prop:text").get();

            let mut text = match prop {
                Some(Value::YText(text)) => text,
                _ => panic!("没有找到Text"),
            };

            let count = Arc::new(Mutex::new(0));

            let (send, mut recv) = channel::<String>(1024);
            let id2 = id2.clone();

            let doc = doc.clone();
            tokio::spawn(async move {
                while let Some(msg) = recv.recv().await {
                    let query = DocQuery::new(doc.clone());
                    let block = query.blocks().id(id2.as_str());
                    let prop = block.and_then(|block| block.prop("prop:text").get());
                    let text = match prop {
                        Some(Value::YText(text)) => text,
                        _ => panic!("没有找到Text"),
                    };
                    let mut txn = doc.transact_mut();
                    text.push(&mut txn, msg.as_str());
                }
            });

            text.observe(move |txn, event| {
                let text = event.target().get_string(txn);
                let mut count = count.lock().unwrap();
                if text.ends_with(format!("ping_{} ", *count).as_str()) {
                    *count += 1;
                    let count = count.clone();
                    let send = send.clone();
                    tokio::spawn(async move {
                        let _ = send.send(format!("pong_{} ", count)).await;
                    });
                }
            });
        });

        loop {
            println!("任务运行中...");
            // 假设这里有一些有用的工作。
            sleep(Duration::from_secs(1)).await;
        }
    });

    handle1.await.expect("Task1 panicked");
    handle2.await.expect("Task2 panicked");

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
