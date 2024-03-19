use std::{rc::Rc, sync::Arc};

use yrs::{
    block::Prelim, Any, Array, ArrayPrelim, Doc, Map, MapPrelim, MapRef, TextPrelim, Transact,
    Transaction, Value,
};

#[derive(Clone)]
pub struct DocQuery {
    doc: Arc<Doc>,
}

#[derive(Clone)]
pub struct BlockMapQuery {
    doc: Arc<Doc>,
    map: MapRef,
}

#[derive(Clone)]
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

    pub fn get_root_note_block(&self) -> Option<BlockQuery> {
        self.flavour("wq:note").first()
    }

    pub fn add_block<F>(&self, id: &str, flavor: &str, init: F) -> BlockQuery
    where
        F: FnOnce(yrs::TransactionMut<'_>, MapRef) + 'static,
    {
        let note = self.get_root_note_block();
        if note.is_none() {
            panic!("没有找到根节点");
        }
        let mut txn = self.doc.transact_mut();
        let map: MapRef = self.map.insert(
            &mut txn,
            id,
            MapPrelim::from([
                ("sys:id".to_string(), Any::String(id.into())),
                ("sys:version".to_string(), Any::Number(1.into())),
                ("sys:flavour".to_string(), Any::String(flavor.into())),
            ]),
        );

        map.insert(&mut txn, "sys:children", ArrayPrelim::default());

        let children = note.unwrap().block.get(&txn, "sys:children");

        match children {
            Some(Value::YArray(array)) => array.insert(&mut txn, 0, Any::String(id.into())),
            _ => panic!("插入失败"),
        };

        init(txn, map);

        BlockQuery {
            doc: self.doc.clone(),
            block: match self.map.get(&self.doc.transact(), id) {
                Some(Value::YMap(block)) => block,
                _ => panic!("插入失败"),
            },
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
