mod client;
mod id;
mod message;
mod model;
mod query;
mod thread;
mod util;

use std::sync::Mutex;

use tokio::time::interval;
use tokio::{time::sleep, time::Duration};

use client::{DocClient, DocInfo};
use yrs::types::text::YChange;
use yrs::types::Attrs;
use yrs::{Any, Text, Transact, Value};
use yrs::{GetString, Map};

use crate::query::query::DocQuery;

#[tokio::main]
async fn main() {
    let info = DocInfo {
        token: "167sD7a3eiDL3DZ4zjOrjzlGQ2g".to_string(),
        document_code: "Page::50304982168064::DEFAULT_PAGE".to_string(),
        url: "ws://10.5.23.192:8896/aio".to_string(),
    };

    let client = DocClient::new(info);

    let handle = tokio::spawn(async move {
        client
            .init(|doc| {
                println!("同步完成");
                {
                    tokio::spawn(async move {
                        let mut interval_timer = interval(Duration::from_secs_f64(1f64));
                        let mut count = 0;

                        loop {
                            interval_timer.tick().await; // 等待下一个间隔
                                                         // 准备你要发送的二进制消息
                            count += 1;
                            if count > 100 {
                                break;
                            }
                            let query = DocQuery::new(doc.clone());
                            let paragraph = query.blocks().flavour("wq:paragraph").first().unwrap();
                            let firstParagraph = match paragraph.prop("prop:text").get() {
                                Some(Value::YText(text)) => Some(text),
                                _ => None,
                            };
                            {
                                let mut txn = doc.transact_mut();
                                let bold = Attrs::from([("bold".into(), true.into())]);

                                if let Some(a) = firstParagraph {
                                    a.insert(&mut txn, 2, "aa");
                                    a.format(&mut txn, 2, 2, bold)
                                }
                            }
                        }
                    });
                }
            })
            .await;
    });

    handle.await.expect("Task panicked");

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
