use std::sync::Arc;
use tokio::sync::{broadcast, Mutex as AsyncMutex};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use yrs::{Doc, Map, MapRef, ReadTxn, Transact, Transaction, Value};

use crate::{
    message::{MessageEncode, SyncStepOneMessage},
    new::block::BlockArray,
};

use super::{block::Block, connection::YjsConnection};

pub struct Document {
    subscripts: Vec<yrs::Subscription>,
    doc: Arc<AsyncMutex<Doc>>,
}

impl YjsConnection {
    pub async fn get_document(mut self) -> Document {
        let doc: Arc<AsyncMutex<Doc>> = Arc::new(AsyncMutex::new(Doc::new()));
        let subscripts = vec![];

        {
            let doc = doc.clone();
            tokio::spawn(async move {
                {
                    let msg = SyncStepOneMessage {
                        document_code: self.info.document_code.clone(),
                    };
                    let doc = doc.lock().await;
                    if self
                        .ws_sender
                        .send(WsMessage::Binary(msg.encode(&doc)))
                        .is_err()
                    {
                        println!("Failed to send step 1 message to websocket");
                    }
                }

                while let Some(msg) = self.yjs_rx.recv().await {
                    let doc = doc.lock().await;
                    let response =
                        msg.message_type
                            .handle_message(&self.info.document_code, &doc, "");
                    drop(doc);
                    if let Some(response) = response {
                        if self.ws_sender.send(WsMessage::Binary(response)).is_err() {
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
    pub fn get_blocks_with_txn<'a, T: ReadTxn + 'a>(txn: &'a T) -> Vec<MapRef> {
        let blocks_map = txn.get_map("blocks");
        if blocks_map.is_none() {
            return vec![];
        }
        let blocks = blocks_map
            .unwrap()
            .iter(txn)
            .map(|(_, block)| match block {
                Value::YMap(block) => {
                    return Some(block);
                }
                _ => None,
            })
            .filter(|block| block.is_some())
            .map(|block| block.unwrap())
            .collect::<Vec<_>>();

        blocks
    }

    pub async fn get_blocks_listener(
        &mut self,
    ) -> (broadcast::Receiver<Vec<MapRef>>, yrs::Subscription) {
        let doc = self.doc.lock().await;

        let (tx, rx) = broadcast::channel::<Vec<MapRef>>(2);

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
        P: Fn(BlockArray, Transaction) -> Option<Block>,
    {
        let mut listener = self.get_blocks_listener().await;

        while let Ok(blocks) = listener.0.recv().await {
            let blocks = BlockArray(blocks);
            let doc = self.doc.lock().await;
            let txn = doc.transact();
            if let Some(block) = predicate(blocks, txn) {
                return Some(block);
            };
        }

        None
    }
}
