use anyhow::Context;
use connection::create_yjs_connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use yrs::{
    Any, Array, ArrayPrelim, Doc, GetString, Map, MapPrelim, MapRef, Observable, Subscription,
    TextRef,
};
use yrs::{Text, TextPrelim, Transact, Value};

use tokio::sync::{self, Mutex as AsyncMutex};

use crate::connection;

fn find_text_ref<T: yrs::ReadTxn>(txn: &T, block_map: &MapRef, key: &str) -> Option<TextRef> {
    let block = block_map.get(txn, key);
    let block = match block {
        Some(Value::YMap(block)) => block,
        _ => {
            // println!("没有找到 Text Ref");
            return None;
        }
    };

    let text_ref = block.get(txn, "prop:text");
    let text_ref = match text_ref {
        Some(Value::YText(text)) => text,
        _ => {
            // println!("没有找到 Text");
            return None;
        }
    };

    Some(text_ref)
}

#[derive(Debug, Clone)]
pub struct DocInfo {
    pub token: String,
    pub document_code: String,
    pub url: String,
}

pub async fn create_peer(
    info: DocInfo,
    target_count: usize,
    id: String,
    thread_id: usize,
    write_delay: usize,
) -> () {
    let id = Arc::new(id);
    println!("id: {}", id.clone());

    let mut handles = Vec::new();

    let subscripts = Arc::new(Mutex::<Vec<Subscription>>::new(vec![]));

    {
        let count = Arc::new(Mutex::new(0));

        let doc = Arc::new(AsyncMutex::new(Doc::new()));
        let id_clone = id.clone();

        let count_clone = count.clone();
        let handle1 = tokio::spawn(create_yjs_connection(
            Arc::clone(&doc),
            info.clone(),
            String::from(format!("Thread_{thread_id}_1")),
            Box::new(move |doc| {
                let doc = Arc::clone(&doc);
                let id = id_clone.clone();
                let subscripts = Arc::clone(&subscripts);
                let count_clone = count_clone.clone();
                tokio::spawn(async move {
                    let doc_clone: tokio::sync::MutexGuard<'_, Doc> = doc.lock().await;
                    let block_map = doc_clone.get_or_insert_map("blocks");

                    let mut txn = doc_clone.transact_mut();
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
                                    return flavor == "wq:note".into();
                                }
                                _ => false,
                            });

                    if note_block.is_none() {
                        println!("没有找到根节点");
                        return;
                    }

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
                    let text_ref = map.insert(
                        &mut txn,
                        "prop:text",
                        TextPrelim::new(format!("ping_{}_0 ", thread_id)),
                    );
                    map.insert(&mut txn, "sys:children", ArrayPrelim::default());

                    if let Some(Value::YArray(array)) =
                        note_block.unwrap().get(&txn, "sys:children")
                    {
                        let len = array.len(&txn);
                        array.insert(&mut txn, len, Any::String(id.clone().as_str().into()));
                    } else {
                        println!("没有找到 Note 节点的 children");
                    }

                    {
                        let (tx, mut rx) = sync::mpsc::channel::<String>(1);
                        let count_clone = Arc::clone(&count_clone);
                        let tx = Arc::new(Mutex::new(tx));
                        let count_clone2 = count_clone.clone();

                        let subscripts_clone = Arc::clone(&subscripts);
                        let sub = text_ref.observe(move |txn, event| {
                            let text_ref = event.target();
                            let text = text_ref.get_string(txn);
                            let mut count_clone = count_clone2.lock().unwrap();
                            // println!("观察到文字更新: Thread_{}: {}", thread_id, *count_clone);

                            if target_count <= *count_clone {
                                println!("{} 完成", thread_id);
                                let mut subscripts = subscripts_clone.lock().unwrap();
                                subscripts.clear();
                                let tx = tx.lock();
                                drop(tx);
                                return;
                            }

                            if text
                                .ends_with(format!("pong_{}_{} ", thread_id, *count_clone).as_str())
                            {
                                *count_clone += 1;

                                if let Err(e) = tx
                                    .lock()
                                    .unwrap()
                                    .try_send(format!("ping_{}_{} ", thread_id, *count_clone))
                                {
                                    println!("添加文字发送消息失败: {:?}", e);
                                }
                            }
                        });
                        let mut subscripts = subscripts.lock().unwrap();
                        subscripts.push(sub);

                        let doc_clone = Arc::clone(&doc);
                        let id_clone = id.clone();
                        tokio::spawn(async move {
                            while let Some(msg) = rx.recv().await {
                                let doc = doc_clone.lock().await;
                                if write_delay > 0 {
                                    sleep(Duration::from_millis(write_delay as u64)).await;
                                }
                                let block_map = doc.get_or_insert_map("blocks");
                                let mut txn = doc.transact_mut();

                                let text_ref = find_text_ref(&txn, &block_map, id_clone.as_str());
                                if let Some(text_ref) = text_ref {
                                    text_ref.push(&mut txn, msg.as_str());
                                }
                            }
                            println!("{}_1 关闭通道", thread_id);
                        });
                    }
                });
            }),
        ));

        handles.push(handle1);
    }

    {
        let count = Arc::new(Mutex::new(0));

        let doc = Arc::new(AsyncMutex::new(Doc::new()));
        let subscripts = Arc::new(Mutex::<Vec<Subscription>>::new(vec![]));
        let id_clone = id.clone();

        let subscripts = Arc::clone(&subscripts);
        let count_clone = count.clone();

        let handle2 = tokio::spawn(create_yjs_connection(
            Arc::clone(&doc),
            info.clone(),
            String::from(format!("Thread_{}_2", thread_id)),
            Box::new(move |doc| {
                let doc = Arc::clone(&doc);
                let id = id_clone.clone();
                let subscripts = Arc::clone(&subscripts);
                let count = count_clone.clone();
                tokio::spawn(async move {
                    println!("同步完成 Thread_{}_2", thread_id);
                    let doc_clone: tokio::sync::MutexGuard<'_, Doc> = doc.lock().await;
                    let block_map = doc_clone.get_or_insert_map("blocks");

                    let (tx, mut rx) = sync::mpsc::channel(1);
                    let tx = Arc::new(Mutex::new(tx));

                    {
                        let mut subscripts = subscripts.lock().unwrap();
                        let id_clone = id.clone();
                        subscripts.push(block_map.observe(move |txn, e| {
                            let block_map = e.target();
                            let block = block_map.get(txn, id_clone.clone().as_str());
                            let block = match block {
                                Some(Value::YMap(block)) => block,
                                _ => {
                                    // println!("没有找到 Paragraph Block");
                                    return;
                                }
                            };

                            let text_ref = block.get(txn, "prop:text");
                            match text_ref {
                                Some(Value::YText(text)) => text,
                                _ => {
                                    println!("没有找到 Text");
                                    return;
                                }
                            };

                            let tx = tx.lock().unwrap();

                            if !tx.is_closed() {
                                let _ = tx.try_send(true).context("发送消息失败");
                                drop(tx);
                            }
                        }));
                    }

                    let count_clone = count.clone();
                    let doc_clone = Arc::clone(&doc);
                    let id_clone = id.clone();

                    tokio::spawn(async move {
                        if let Some(success) = rx.recv().await {
                            if success {
                                drop(rx);
                            }
                            let doc = doc_clone.lock().await;

                            let block_map = doc.get_or_insert_map("blocks");
                            let txn = doc.transact();
                            let text_ref = find_text_ref(&txn, &block_map, id_clone.as_str());

                            if let Some(text_ref) = text_ref {
                                let (tx, mut rx) = sync::mpsc::channel::<String>(1);

                                let tx = Arc::new(Mutex::new(tx));

                                let tx_clone = Arc::clone(&tx);
                                let subscripts_clone = Arc::clone(&subscripts);
                                let sub = text_ref.observe(move |txn, event| {
                                    let text_ref = event.target();
                                    let text = text_ref.get_string(txn);

                                    let mut count_clone = count_clone.lock().unwrap();
                                    // println!(
                                    //     "观察到文字更新: Thread_{}_2: {}  ",
                                    //     thread_id, *count_clone
                                    // );

                                    if target_count <= *count_clone {
                                        println!("{} 完成", thread_id);
                                        let mut subscripts = subscripts_clone.lock().unwrap();
                                        subscripts.clear();
                                        let tx = tx_clone.lock();
                                        drop(tx);
                                        return;
                                    }

                                    if text.ends_with(
                                        format!("ping_{}_{} ", thread_id, *count_clone).as_str(),
                                    ) {
                                        if let Err(e) = tx_clone.lock().unwrap().try_send(format!(
                                            "pong_{}_{} ",
                                            thread_id, *count_clone
                                        )) {
                                            println!("添加文字发送消息失败: {:?}", e);
                                        }
                                        *count_clone += 1;
                                    }
                                });

                                {
                                    let count_clone = count.clone();
                                    let text_content = text_ref.get_string(&txn);

                                    if text_content
                                        .ends_with(format!("ping_{}_0 ", thread_id).as_str())
                                    {
                                        let mut count_clone = count_clone.lock().unwrap();
                                        if let Err(e) = tx
                                            .lock()
                                            .unwrap()
                                            .try_send(format!("pong_{}_0 ", thread_id).to_string())
                                        {
                                            println!("添加文字发送消息失败: {:?}", e);
                                        }
                                        *count_clone += 1;
                                    }
                                }

                                let mut subscripts = subscripts.lock().unwrap();
                                subscripts.push(sub);

                                let doc = Arc::clone(&doc_clone);
                                let id_clone = id.clone();
                                tokio::spawn(async move {
                                    while let Some(msg) = rx.recv().await {
                                        let doc = doc.lock().await;
                                        if write_delay > 0 {
                                            sleep(Duration::from_millis(write_delay as u64)).await;
                                        }
                                        let block_map = doc.get_or_insert_map("blocks");

                                        let mut txn = doc.transact_mut();
                                        let text_ref =
                                            find_text_ref(&txn, &block_map, id_clone.as_str());
                                        if let Some(text_ref) = text_ref {
                                            text_ref.push(&mut txn, msg.as_str());
                                        }
                                    }
                                    println!("{}_2 关闭通道", thread_id);
                                });
                            };
                        }
                    });
                });
            }),
        ));

        handles.push(handle2);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}

pub async fn create_direct(
    info: DocInfo,
    target_count: usize,
    id: String,
    thread_id: usize,
    write_delay: usize,
) {
    let id = Arc::new(id);
    println!("id: {}", id.clone());

    let mut handles = Vec::new();

    {
        let count = Arc::new(Mutex::new(0));

        let doc = Arc::new(AsyncMutex::new(Doc::new()));
        let id_clone = id.clone();

        let count_clone = count.clone();
        let handle1 = tokio::spawn(create_yjs_connection(
            Arc::clone(&doc),
            info.clone(),
            String::from(format!("Thread_{thread_id}_1")),
            Box::new(move |doc| {
                let doc = Arc::clone(&doc);
                let id = id_clone.clone();
                let count_clone = count_clone.clone();
                tokio::spawn(async move {
                    let doc_clone: tokio::sync::MutexGuard<'_, Doc> = doc.lock().await;
                    let block_map = doc_clone.get_or_insert_map("blocks");

                    let mut txn = doc_clone.transact_mut();
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
                                    return flavor == "wq:note".into();
                                }
                                _ => false,
                            });

                    if note_block.is_none() {
                        println!("没有找到根节点");
                        return;
                    }

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
                    let _ = map.insert(
                        &mut txn,
                        "prop:text",
                        TextPrelim::new(format!("direct_1_{}_0 ", thread_id)),
                    );
                    map.insert(&mut txn, "sys:children", ArrayPrelim::default());

                    if let Some(Value::YArray(array)) =
                        note_block.unwrap().get(&txn, "sys:children")
                    {
                        let len = array.len(&txn);
                        array.insert(&mut txn, len, Any::String(id.clone().as_str().into()));
                    } else {
                        println!("没有找到 Note 节点的 children");
                    }

                    let doc_clone = Arc::clone(&doc);
                    let id_clone = id.clone();
                    tokio::spawn(async move {
                        loop {
                            if write_delay > 0 {
                                sleep(Duration::from_millis(write_delay as u64)).await;
                            }
                            let doc = doc_clone.lock().await;

                            let mut count_clone = count_clone.lock().unwrap();

                            if target_count <= *count_clone {
                                println!("{} 完成", thread_id);
                                return;
                            }

                            *count_clone += 1;

                            let block_map = doc.get_or_insert_map("blocks");

                            let mut txn = doc.transact_mut();
                            let text_ref = find_text_ref(&txn, &block_map, id_clone.as_str());
                            if let Some(text_ref) = text_ref {
                                text_ref.push(
                                    &mut txn,
                                    format!("direct_1_{}_{} ", thread_id, *count_clone).as_str(),
                                );
                            }
                        }
                    });
                });
            }),
        ));

        handles.push(handle1);
    }

    {
        let count = Arc::new(Mutex::new(0));

        let doc = Arc::new(AsyncMutex::new(Doc::new()));
        let id_clone = id.clone();

        let count_clone = count.clone();
        let handle2 = tokio::spawn(create_yjs_connection(
            Arc::clone(&doc),
            info.clone(),
            String::from(format!("Thread_{thread_id}_2")),
            Box::new(move |doc| {
                let doc = Arc::clone(&doc);
                let id = id_clone.clone();
                let count_clone = count_clone.clone();
                tokio::spawn(async move {
                    let doc_clone = Arc::clone(&doc);
                    let id_clone = id.clone();
                    tokio::spawn(async move {
                        loop {
                            if write_delay > 0 {
                                sleep(Duration::from_millis(write_delay as u64)).await;
                            }
                            let doc = doc_clone.lock().await;

                            let mut count_clone = count_clone.lock().unwrap();

                            if target_count <= *count_clone {
                                println!("完成");
                                return;
                            }

                            let block_map = doc.get_or_insert_map("blocks");
                            let mut txn = doc.transact_mut();
                            let text_ref = find_text_ref(&txn, &block_map, id_clone.as_str());
                            if let Some(text_ref) = text_ref {
                                *count_clone += 1;

                                text_ref.push(
                                    &mut txn,
                                    format!("direct_2_{}_{} ", thread_id, *count_clone).as_str(),
                                );
                            }
                        }
                    });
                });
            }),
        ));

        handles.push(handle2);
    }

    for handle in handles {
        handle.await.expect("Task panicked");
    }
}
