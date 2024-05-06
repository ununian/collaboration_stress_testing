use anyhow::Result;
use collaboration_stress_testing::{new::connection::new_create_yjs_connection, peer::DocInfo};

#[tokio::main]
async fn main() -> Result<()> {
    let info = DocInfo {
        token: "BmIaTq6J8h2zT1AvClW8ufTatV8".to_string(),
        document_code: "Page::51365455853056::DEFAULT_PAGE".to_string(),
        url: "ws://10.5.23.192:8896/aio".to_string(),
    };
    let a = new_create_yjs_connection(&info).await;

    Ok(())
}
