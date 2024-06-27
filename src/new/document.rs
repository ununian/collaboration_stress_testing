use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use yrs::{Doc, Map, MapRef, ReadTxn, Subscription, Transact, Transaction, Value};

use crate::{
    message::{MessageEncode, SyncStepOneMessage, UpdateMessage},
    new::block::BlockMap,
};

use super::{block::Block, connection::YjsConnection};

pub struct Document {
    subscripts: Vec<yrs::Subscription>,
    pub doc: Arc<AsyncMutex<Doc>>,
}

impl YjsConnection {
    pub async fn get_document(mut self) -> Document {
        let doc: Arc<AsyncMutex<Doc>> = Arc::new(AsyncMutex::new(Doc::new()));
        let mut subscripts = vec![];
        let sender = Arc::new(Mutex::new(self.ws_sender));

        {
            let doc = doc.clone();
            let sender = sender.clone();
            {
                let msg = SyncStepOneMessage {
                    document_code: self.info.document_code.clone(),
                };
                let doc = doc.try_lock().unwrap();
                if sender
                    .lock()
                    .unwrap()
                    .send(WsMessage::Binary(msg.encode(&doc)))
                    .is_err()
                {
                    println!("Failed to send step 1 message to websocket");
                }
            }

            let document_code = Arc::new(self.info.document_code.clone());
            let document_code = document_code.clone();
            let doc = doc.try_lock().unwrap();

            subscripts.push(
                doc.observe_update_v1(move |_, event| {
                    println!("观察到更新");
                    let msg = UpdateMessage {
                        update: event.update.clone(),
                        document_code: document_code.to_string(),
                    }
                    .encode();

                    let sender = sender.lock();
                    if sender.is_err() {
                        return;
                    }
                    if sender.unwrap().send(WsMessage::Binary(msg)).is_err() {
                        println!("Failed to send step 1 message to websocket");
                    }
                })
                .unwrap(),
            );
        }

        {
            let doc = doc.clone();
            tokio::spawn(async move {
                while let Some(msg) = self.yjs_rx.recv().await {
                    let doc = doc.lock().await;
                    let response =
                        msg.message_type
                            .handle_message(&self.info.document_code, &doc, "");
                    drop(doc);
                    if let Some(response) = response {
                        if sender
                            .lock()
                            .unwrap()
                            .send(WsMessage::Binary(response))
                            .is_err()
                        {
                            println!("Failed to send message to websocket");
                        }
                    }
                }
            });
        }

        Document { subscripts, doc }
    }
}

impl Document {
    pub fn get_blocks_with_txn<'a, T: ReadTxn + 'a>(txn: &'a T) -> HashMap<String, MapRef> {
        let blocks_map = txn.get_map("blocks");
        if blocks_map.is_none() {
            return HashMap::new();
        }

        let mut blocks = HashMap::new();
        blocks_map
            .unwrap()
            .iter(txn)
            .map(|(id, block)| match block {
                Value::YMap(block) => {
                    return Some((id, block));
                }
                _ => None,
            })
            .filter(|block| block.is_some())
            .map(|block| block.unwrap())
            .for_each(|block| {
                blocks.insert(block.0.to_string(), block.1);
            });

        blocks
    }

    pub async fn get_blocks_listener(
        &mut self,
    ) -> (
        broadcast::Receiver<HashMap<String, MapRef>>,
        yrs::Subscription,
    ) {
        let doc = self.doc.lock().await;

        let (tx, rx) = broadcast::channel::<HashMap<String, MapRef>>(2);

        {
            // first time
            let txn = doc.transact();
            tx.send(Self::get_blocks_with_txn(&txn)).unwrap();
        }

        let sub = doc
            .observe_after_transaction(move |txn| {
                tx.send(Self::get_blocks_with_txn(txn)).unwrap();
            })
            .unwrap();

        (rx, sub)
    }

    pub async fn wait_for_block<P>(&mut self, predicate: P) -> Option<Block>
    where
        P: Fn(BlockMap, Transaction) -> Option<Block>,
    {
        let mut listener = self.get_blocks_listener().await;

        while let Ok(blocks) = listener.0.recv().await {
            let blocks = BlockMap(blocks);
            let doc = self.doc.lock().await;
            let txn = doc.transact();
            if let Some(block) = predicate(blocks, txn) {
                return Some(block);
            };
        }

        None
    }
}
