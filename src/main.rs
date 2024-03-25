mod connection;
mod id;
mod message;
mod model;
mod peer;
mod query;
mod util;

use clap::Parser;

use nanoid::nanoid;
use peer::{create_peer, DocInfo};
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

    #[arg(help = "文档Code,用空格分隔多个文档编码")]
    document_code: Vec<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut handles = Vec::new();

    for code in cli.document_code {
        let info = DocInfo {
            token: cli.token.clone(),
            document_code: format!("Page::{}::DEFAULT_PAGE", code),
            url: cli.url.clone(),
        };

        for _ in 0..cli.peer_num {
            let handle = tokio::spawn(create_peer(info.clone(), cli.write_num, nanoid!(), 0));
            handles.push(handle);
            sleep(Duration::from_secs_f32(0.5f32)).await;
        }
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }

    sleep(Duration::from_secs(1)).await;
}
