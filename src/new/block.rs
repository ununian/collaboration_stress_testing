use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use yrs::{Any, Array, ArrayPrelim, Map, MapPrelim, ReadTxn, TransactionMut, Value, WriteTxn};

#[derive(Debug)]
pub struct BlockMap(pub HashMap<String, yrs::MapRef>);

#[derive(Debug, Clone)]
pub struct Block(pub yrs::MapRef);

impl BlockMap {
    pub fn root<'a, T: ReadTxn + 'a>(&self, txn: &'a T) -> Option<Block> {
        let block = self
            .0
            .iter()
            .find(|block| match block.1.get(txn, "sys:flavour") {
                Some(Value::Any(Any::String(flavor))) => {
                    return flavor == "wq:page".into();
                }
                _ => false,
            });

        match block {
            Some(block) => Some(Block(block.1.clone())),
            None => None,
        }
    }
}

impl Block {
    pub fn flavor<'b, T: ReadTxn + 'b>(&self, txn: &'b T) -> Arc<str> {
        match self.0.get(txn, "sys:flavour") {
            Some(Value::Any(Any::String(flavor))) => flavor,
            _ => panic!("Block does not have a flavor"),
        }
    }

    pub fn id<'b, T: ReadTxn + 'b>(&self, txn: &'b T) -> Arc<str> {
        match self.0.get(txn, "sys:id") {
            Some(Value::Any(Any::String(id))) => id,
            _ => panic!("Block does not have an id"),
        }
    }

    pub fn get_children<'b, T: ReadTxn + 'b>(&self, txn: &'b T, arr: &BlockMap) -> Vec<Block> {
        let children = match self.0.get(txn, "sys:children") {
            Some(Value::YArray(children)) => children,
            _ => return vec![],
        };

        let children = children
            .iter(txn)
            .flat_map(|item| match item {
                Value::Any(Any::String(id)) => Some(id),
                _ => None,
            })
            .collect::<Vec<_>>();

        children
            .iter()
            .filter_map(|id| {
                arr.0
                    .iter()
                    .find(|block| match block.1.get(txn, "sys:id") {
                        Some(Value::Any(Any::String(block_id))) => block_id == *id,
                        _ => false,
                    })
                    .map(|block| Block(block.1.clone()))
            })
            .collect()
    }

    pub fn get_child_by_id<'b, T: ReadTxn + 'b>(
        &self,
        txn: &'b T,
        arr: &BlockMap,
        id: &str,
    ) -> Option<Block> {
        arr.0
            .iter()
            .find(|block| match block.1.get(txn, "sys:id") {
                Some(Value::Any(Any::String(block_id))) => *block_id == *id,
                _ => false,
            })
            .map(|block| Block(block.1.clone()))
    }

    pub fn get_children_by_flavor<'b, T: ReadTxn + 'b>(
        &self,
        txn: &'b T,
        arr: &BlockMap,
        flavor: &str,
    ) -> Vec<Block> {
        self.get_children(txn, arr)
            .into_iter()
            .filter(|block| match block.0.get(txn, "sys:flavour") {
                Some(Value::Any(Any::String(block_flavor))) => *block_flavor == *flavor,
                _ => false,
            })
            .map(|block| Block(block.0.clone()))
            .collect()
    }

    pub fn get_text<'b, T: ReadTxn + 'b>(&self, txn: &'b T) -> Option<yrs::TextRef> {
        match self.0.get(txn, "prop:text") {
            Some(Value::YText(text)) => Some(text),
            _ => None,
        }
    }

    pub fn insert(
        &self,
        txn: &mut TransactionMut,
        flavor: &str,
        id: &str,
        index: Option<u32>,
    ) -> Option<Block> {
        let blocks = txn.get_or_insert_map("blocks");

        let block = blocks.insert(
            txn,
            id,
            MapPrelim::from([
                ("sys:id".to_string(), Any::String(id.into())),
                ("sys:version".to_string(), Any::Number(1.into())),
                ("sys:flavour".to_string(), Any::String(flavor.into())),
            ]),
        );

        block.insert(txn, "sys:children", ArrayPrelim::default());

        let parent_children = match self.0.get(txn, "sys:children") {
            Some(Value::YArray(children)) => children,
            _ => return None,
        };

        parent_children.insert(
            txn,
            index.unwrap_or(parent_children.len(txn)),
            Any::String(id.into()),
        );

        Some(Block(block))
    }
}
