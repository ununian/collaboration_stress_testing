use anyhow::Result;
use collaboration_stress_testing::peer::DocInfo;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let info = DocInfo {
        token: "FIz7tuH7t576p2J9p8qM4b3Dmkg".to_string(),
        document_code: "Page::51582114155008::DEFAULT_PAGE".to_string(),
        url: "ws://10.5.23.192:8896/aio".to_string(),
    };
    let a = info.connection().await;
    let mut a = a.get_document().await;

    let note = a
        .wait_for_block(|arr, txn| {
            let root = arr.root(&txn);

            if let Some(root) = root {
                let children = root.get_children(&arr, &txn);

                let note = children
                    .iter()
                    .find(|block| block.flavor(&txn) == "wq:note".into());

                if let Some(note) = note {
                    return Some(note.clone());
                }
            }

            return None;
        })
        .await;

    if note.is_none() {
        println!("未找到笔记");
        return Ok(());
    }

    let note = note.unwrap();

    println!("---{:?}", note);

    signal::ctrl_c().await.expect("failed to listen for event");
    Ok(())
}
