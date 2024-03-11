mod message;

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use message::AuthenticationMessage;
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use yrs::{Doc, GetString, ReadTxn, StateVector, Text, Transact, Update};

use crate::message::{
    insert_content_message, next_step, IncomingMessage, MessageDecode, MessageEncode, MessageType,
};
struct DocInfo {
    token: String,
    document_code: String,
    url: String,
    target_block_id: String,
}

struct Client {
    pub doc: Doc,
}

#[tokio::main]
async fn main() {
    // This is running on a core thread.
    let info = DocInfo {
        token: "fEecue-ZXhhvcyLLHN9eEH7bB9I".to_string(),
        document_code: "Page::50125935323648::DEFAULT_PAGE".to_string(),
        url: "ws://127.0.0.1:3110".to_string(),
        target_block_id: "Et87mcQWLx".to_string(),
    };

    let blocking_task = tokio::task::spawn_blocking(|| {
        // This is running on a blocking thread.
        // Blocking here is ok.
    });

    // We can wait for the blocking task like this:
    // If the blocking task panics, the unwrap below will propagate the
    // panic.
    start_client(&info).await;
    blocking_task.await.unwrap();
}

fn to_hex_string(data: &Vec<u8>) -> String {
    let hex_groups = data
        .chunks(2) // 每2个字节作为一组
        .map(|chunk| {
            chunk
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect::<String>()
        })
        .collect::<Vec<_>>();

    // 每8组（总共16个字节）换行
    hex_groups
        .chunks(8) // 每8个组为一行
        .map(|line| line.join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn start_client(info: &DocInfo) {
    let doc = Doc::new();
    let mut client = Client { doc };

    // WebSocket 服务地址

    // 连接到 WebSocket 服务
    let (ws_stream, _) = connect_async(&info.url).await.expect("Failed to connect");
    println!("WebSocket client connected");

    // 分离发送与接收处理器
    let (mut write, mut read) = ws_stream.split();

    let auth_message = AuthenticationMessage {
        token: (info.token).clone(),
        document_code: info.document_code.clone(),
    };

    write
        .send(Message::Binary(auth_message.encode()))
        .await
        .expect("Failed to send message");

    loop {
        // 等待并接收消息
        match read.next().await {
            Some(Ok(message)) => {
                match message {
                    // 仅处理二进制消息
                    Message::Binary(bin) => {
                        println!("{}", to_hex_string(&bin));
                        let msg = IncomingMessage::decode(&bin);
                        println!(
                            "{:?}, {:?} {} {}",
                            msg.document_code,
                            msg.message_type,
                            msg.cur.buf.len(),
                            msg.cur.next
                        );

                        match next_step(&msg.message_type, &info, &mut client) {
                            Some(buf) => {
                                if buf.len() == 0 {
                                    break;
                                }
                                let response_message = Message::Binary(buf);
                                match write.send(response_message).await {
                                    Ok(_) => println!("Sent a response binary message"),
                                    Err(e) => {
                                        eprintln!("Failed to send message: {}", e);
                                        break; // 出错时退出循环
                                    }
                                }
                            }
                            None => {
                                continue;
                            }
                        }
                        // 响应发送二进制消息
                        // let response_message = Message::Binary(vec![6, 7, 8, 9, 0]); // 举例，你应根据需要修改此处
                        // match write.send(response_message).await {
                        //     Ok(_) => println!("Sent a response binary message"),
                        //     Err(e) => {
                        //         eprintln!("Failed to send message: {}", e);
                        //         break; // 出错时退出循环
                        //     }
                        // }
                    }
                    _ => println!("Received a non-binary message"),
                }
            }
            Some(Err(e)) => {
                eprintln!("Error during receiving message: {}", e);
                break; // 出错时退出循环
            }
            None => break, // 无更多消息时退出循环
        }
    }

    let mut interval_timer = interval(Duration::from_secs_f64(0.1));
    let mut count = 0;
    loop {
        interval_timer.tick().await; // 等待下一个间隔
                                     // 准备你要发送的二进制消息
        count += 1;
        if count > 300 {
            break;
        }
        let buf = insert_content_message(&info, &mut client).unwrap();

        let response_message = Message::Binary(buf);
        match write.send(response_message).await {
            Ok(_) => println!("Sent a response binary message"),
            Err(e) => {
                eprintln!("Failed to send message: {}", e);
                return;
            }
        }
    }

    // 给服务器一些时间来回应后再关闭连接
    sleep(Duration::from_secs(1)).await;
}
