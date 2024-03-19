use std::{rc::Rc, sync::Arc};

use yrs::{block::Prelim, Any, Doc, Map, MapRef, Transact, Transaction, Value};

pub struct DocQuery {
    doc: Arc<Doc>,
}

pub struct BlockMapQuery {
    doc: Arc<Doc>,
    map: MapRef,
}

pub struct BlocksQuery {
    doc: Arc<Doc>,
    blocks: Vec<MapRef>,
}

pub struct BlockQuery {
    doc: Arc<Doc>,
    block: MapRef,
}

impl DocQuery {
    pub fn new(doc: Arc<Doc>) -> Self {
        DocQuery { doc }
    }

    pub fn blocks(&self) -> BlockMapQuery {
        let blocks = self.doc.get_or_insert_map("blocks");
        BlockMapQuery {
            doc: self.doc.clone(),
            map: blocks,
        }
    }
}

/*
let query = BlockMapQuery::new(doc);
let a_paragraph = query.blocks().id("one_id");
let paragraphs: = query.blocks().flavour("wq:paragraph");
*/

impl BlockMapQuery {
    pub fn id(&self, id: &str) -> Option<BlockQuery> {
        match self.map.get(&self.doc.transact(), id) {
            Some(Value::YMap(block)) => Some(BlockQuery {
                doc: self.doc.clone(),
                block,
            }),
            _ => None,
        }
    }

    pub fn flavour(&self, flavour: &str) -> BlocksQuery {
        let txn = self.doc.transact();
        let blocks = self
            .map
            .iter(&txn)
            .filter_map(|(_, v)| match v {
                Value::YMap(block) => {
                    if block.get(&txn, "sys:flavour")
                        == Some(Value::Any(Any::String(flavour.into())))
                    {
                        Some(block)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        BlocksQuery {
            doc: self.doc.clone(),
            blocks,
        }
    }
}

/*
let text: Option<Value> = query.blocks().id("one_id").prop('sys:text').get();
query.blocks().id("one_id").prop('sys:text').get(Value:YText());
*/

impl BlocksQuery {
    pub fn prop(&self, key: &str) -> Vec<PropQuery> {
        self.blocks
            .iter()
            .map(|block| PropQuery {
                doc: self.doc.clone(),
                block: block.clone(),
                key: key.to_string(),
            })
            .collect()
    }

    pub fn first(&self) -> Option<BlockQuery> {
        self.blocks.first().map(|block| BlockQuery {
            doc: self.doc.clone(),
            block: block.clone(),
        })
    }
}

impl BlockQuery {
    pub fn prop(&self, key: &str) -> PropQuery {
        PropQuery {
            doc: self.doc.clone(),
            block: self.block.clone(),
            key: key.to_string(),
        }
    }

    pub fn flavour(&self) -> Option<String> {
        let txn = self.doc.transact();
        self.block.get(&txn, "sys:flavour").and_then(|v| match v {
            Value::Any(Any::String(s)) => Some(s.clone().to_string()),
            _ => None,
        })
    }
}

pub struct PropQuery {
    doc: Arc<Doc>,
    block: MapRef,
    key: String,
}

impl PropQuery {
    pub fn get(&self) -> Option<Value> {
        let txn = self.doc.transact();
        self.block.get(&txn, &self.key)
    }

    pub fn set<T>(&self, value: T)
    where
        T: Prelim,
    {
        let mut txn = self.doc.transact_mut();
        self.block.insert(&mut txn, self.key.as_str(), value);
    }
}
