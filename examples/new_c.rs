use std::sync::{Arc, Mutex};

use anyhow::Result;
use collaboration_stress_testing::{new::document::Document, peer::DocInfo};
use nanoid::nanoid;
use tokio::{signal, sync::mpsc};
use yrs::{GetString, Map, Observable, Text, TextPrelim, TextRef, Transact, Value};
#[tokio::main]
async fn main() -> Result<()> {
    let info = DocInfo {
        token: "9I7TG8_-6A64LFVf87FSiJ-naaQ".to_string(),
        document_code: "Page::52514713528576::DEFAULT_PAGE".to_string(),
        url: "ws://10.5.23.192:8896/aio".to_string(),
    };

    let id = nanoid!();
    let client1 = create_client(info.clone(), id.clone(), true).await?;
    let count = Arc::new(Mutex::new(0));

    let mut subscripts1 = vec![];

    let client1_count = count.clone();
    subscripts1.push(client1.0.observe(|txn, event| {
        let value = event.target().get_string(txn);
        println!("观察到更新11111: {:?}", value);
    }));

    let client2 = create_client(info.clone(), id.clone(), false).await?;
    let mut subscripts2 = vec![];

    let client2_count = count.clone();
    let (tx, mut rx) = mpsc::channel(1);
    let client2_send_text = Arc::new(tx);
    subscripts2.push(client2.0.observe(move |txn, event| {
        let text = event.target();
        let value = text.get_string(txn);

        text.insert(txn, 0, "Pong_0");

        println!("观察到更新22222: {:?}", value);
        let count = client2_count.lock().unwrap();
        if *count >= 500 {
            return;
        }
        if value.ends_with(format!("Ping_{}", *count).as_str()) {
            client2_send_text
                .try_send(format!("Pong_{}", *count))
                .unwrap();
        }
    }));

    // let textArc = Arc::new(Mutex::new(client2.0));
    // let docArc = client2.1.doc.clone();
    // tokio::spawn(async move {
    //     while let Some(text) = rx.recv().await {
    //         let doc = docArc.lock().await;
    //         let mut txn = doc.transact_mut();
    //         textArc.lock().unwrap().insert(&mut txn, 0, text.as_str());
    //     }
    // });

    let doc = client1.1.doc.lock().await;
    {
        let mut txn = doc.transact_mut();
        client1.0.insert(&mut txn, 0, "Ping_0");
    }

    signal::ctrl_c().await.expect("failed to listen for event");
    Ok(())
}

// 创建一个新的客户端，返回指定的 id 的 Paragraph Block 的 TextRef 属性的对象
// isCreateParagraph 表示是否需要客户端创建一个新的 Paragraph Block，如果不需要创建，需要等待另一个客户端创建
async fn create_client(
    info: DocInfo,
    id: String,
    is_create_paragraph: bool,
) -> Result<(TextRef, Document)> {
    let connect = info.connection().await;
    let mut document = connect.get_document().await;

    let note = document
        .wait_for_block(|arr, txn| {
            let root = arr.root(&txn);
            if let Some(root) = root {
                let notes = root.get_children_by_flavor(&txn, &arr, "wq:note");
                if notes.len() == 1 {
                    return Some(notes[0].clone());
                }
            }
            None
        })
        .await;

    let note = note.ok_or_else(|| anyhow::anyhow!("未找到笔记"))?;

    if is_create_paragraph {
        let text = {
            let doc = document.doc.lock().await;
            let mut txn = doc.transact_mut();
            let paragraph = note
                .insert(&mut txn, "wq:paragraph", id.as_str(), None)
                .unwrap();
            paragraph
                .0
                .insert(&mut txn, "prop:text", TextPrelim::new(""))
        };
        return Ok((text, document));
    }

    let paragraph = document
        .wait_for_block(|arr, txn| {
            let root = arr.root(&txn);
            root.and_then(|root| root.get_child_by_id(&txn, &arr, id.as_str()))
        })
        .await
        .ok_or_else(|| anyhow::anyhow!("未找到指定的 Paragraph Block"))?;

    let text = {
        let doc = document.doc.lock().await;
        let txn = doc.transact();
        let text = paragraph.0.get(&txn, "prop:text");
        match text {
            Some(Value::YText(text)) => text,
            Some(_) => return Err(anyhow::anyhow!("TextRef 类型错误")),
            None => return Err(anyhow::anyhow!("未找到指定的 TextRef")),
        }
    };

    Ok((text, document))
}

async fn create_peer(info: DocInfo, id: String) -> Result<()> {
    Ok(())
}
