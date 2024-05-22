use std::sync::Arc;

use yrs::{Any, Array, Map, ReadTxn, Value};

#[derive(Debug)]
pub struct BlockArray(pub Vec<yrs::MapRef>);

#[derive(Debug, Clone)]
pub struct Block(pub yrs::MapRef);

impl BlockArray {
    pub fn root<'a, T: ReadTxn + 'a>(&self, txn: &'a T) -> Option<Block> {
        let block = self
            .0
            .iter()
            .find(|block| match block.get(txn, "sys:flavour") {
                Some(Value::Any(Any::String(flavor))) => {
                    return flavor == "wq:page".into();
                }
                _ => false,
            });

        match block {
            Some(block) => Some(Block(block.clone())),
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

    pub fn get_children<'b, T: ReadTxn + 'b>(&self, arr: &BlockArray, txn: &'b T) -> Vec<Block> {
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

        println!("children: {:?}", children);

        children
            .iter()
            .filter_map(|id| {
                arr.0
                    .iter()
                    .find(|block| match block.get(txn, "sys:id") {
                        Some(Value::Any(Any::String(block_id))) => block_id == *id,
                        _ => false,
                    })
                    .map(|block| Block(block.clone()))
            })
            .collect()
    }

    pub fn get_text<'b, T: ReadTxn + 'b>(&self, txn: &'b T) -> Option<yrs::TextRef> {
        match self.0.get(txn, "text") {
            Some(Value::YText(text)) => Some(text),
            _ => None,
        }
    }
}
