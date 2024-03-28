mod connection;
mod create;
mod id;
mod message;
mod model;
mod peer;
mod query;
mod util;

use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;

use nanoid::nanoid;
use peer::{create_direct, create_peer, DocInfo};
use tokio::{time::sleep, time::Duration};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(
        long,
        help = "协同服务 WebSocket URL",
        default_value = "ws://10.5.23.192:8896/aio"
    )]
    url: String,

    #[arg(
        long,
        help = "用户 Token, 在 LocalStorage 的 __dui_cache_Access-Token 中获取"
    )]
    token: String,

    #[arg(
        long,
        default_value = "10",
        help = "测试线程对的数量, 每个线程对包含两个连接"
    )]
    peer_num: usize,

    #[arg(long, default_value = "10", help = "每个连接写入的数量")]
    write_num: usize,

    #[arg(long, default_value = "500", help = "写入延迟, 毫秒")]
    write_delay: usize,

    #[arg(
        long,
        default_value = "true",
        help = "是否需要观察到对应的文字更新,如果为 false,每个线程会直接写入数据,不会观察到对应的文字更新"
    )]
    need_observe: bool,

    #[arg(
        long,
        help = "创建文档的数量,默认为0,设置了的话 document_code 就无效了",
        default_value = "0"
    )]
    create_doc_num: Option<usize>,

    #[arg(
        long,
        help = "服务器的地址,类似 http://10.5.23.192:8896",
        default_value = "http://10.5.23.192:8896"
    )]
    create_origin: Option<String>,

    #[arg(
        long,
        help = "文档Code的前缀,用于创建文档,如果没有就取当前时间戳的前10位"
    )]
    create_prefix: Option<String>,

    #[arg(help = "文档Code,用空格分隔多个文档编码")]
    document_code: Vec<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut doc_codes = vec![];

    if let Some(create_num) = cli.create_doc_num {
        if create_num > 0 {
            let now = SystemTime::now();

            let time = match now.duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_secs(),
                Err(e) => {
                    eprintln!("Error: {:?}", e);
                    0
                }
            };

            if cli.create_prefix.is_none() {
                println!("Warning: 未设置文档前缀,将使用当前时间戳的前10位, {}", time);
            }

            if cli.create_origin.is_none() {
                println!("Warning: 未设置服务器地址,将使用默认地址, http://10.5.23.192:8896");
            }

            let codes = create::create_docs(
                create_num,
                &cli.create_origin.unwrap_or("".to_string()),
                &cli.create_prefix.unwrap_or(time.to_string()),
                &cli.token,
            )
            .await;
            println!("创建的文档 Code {:?}", codes);

            doc_codes = codes;
        }
    } else {
        println!("Warning: 未设置创建文档数量,将使用 document_code");
        doc_codes = cli.document_code.clone();
    }

    let mut handles = Vec::new();

    for code in doc_codes {
        let info = DocInfo {
            token: cli.token.clone(),
            document_code: format!("Page::{}::DEFAULT_PAGE", code),
            url: cli.url.clone(),
        };

        if cli.need_observe {
            for i in 0..cli.peer_num {
                let handle = tokio::spawn(create_peer(
                    info.clone(),
                    cli.write_num,
                    nanoid!(),
                    i,
                    cli.write_delay,
                ));
                handles.push(handle);
                sleep(Duration::from_secs_f32(0.5f32)).await;
            }
        }

        for i in 0..cli.peer_num {
            let handle = tokio::spawn(create_direct(
                info.clone(),
                cli.write_num,
                nanoid!(),
                i,
                cli.write_delay,
            ));
            handles.push(handle);
            sleep(Duration::from_secs_f32(0.5f32)).await;
        }
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    sleep(Duration::from_secs(1)).await;
}
