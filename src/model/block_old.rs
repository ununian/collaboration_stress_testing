use nanoid::nanoid;
use std::{collections::HashMap, sync::Arc};
use yrs::{
    block::Prelim, Any, ArrayPrelim, Map, MapPrelim, MapRef, TextPrelim, TransactionMut, Value,
};

#[derive(Debug)]
pub struct Block {
    pub id: String,
    pub flavour: String,
    pub version: i64,

    pub children: Vec<String>,

    pub props: HashMap<String, dyn Prelim>,
}

impl Block {
    pub fn new(flavour: String) -> Block {
        Block {
            id: next_id(),
            flavour,
            version: 1,
            children: vec![],
            props: HashMap::new(),
        }
    }

    fn to_base_map(&self) -> MapPrelim<Any> {
        let mut map = HashMap::from([
            ("sys:id".to_string(), Any::String(self.id.as_str().into())),
            (
                "sys:flavour".to_string(),
                Any::String(self.flavour.as_str().into()),
            ),
            ("sys:version".to_string(), Any::Number(self.version as f64)),
        ]);

        MapPrelim::from(map)
    }

    fn insert_block(&self, txn: &mut TransactionMut, parent: &MapRef) {
        let block_map = parent.insert(txn, self.id.as_str(), MapPrelim::from(self.to_base_map()));
        block_map.insert(
            txn,
            "sys:children",
            ArrayPrelim::from(self.children.clone()),
        );

        self.props.iter().for_each(|prop| {
            block_map.insert(txn, "", TextPrelim::new("value"));
        });
    }

    pub fn insert_paragraph(txn: &mut TransactionMut, map: &MapRef) {
        let mut block = Block::new("paragraph".to_string());
    }

    // pub fn to_value(&self) -> Value {
    //     let title = TextPrelim::new("value");
    //     let mut a = MapPrelim::new();

    //     let val = MapPrelim::from([
    //         ("a".to_string(), Value::Any(Any::Number(1f64))),
    //         ("q".to_string(), Value::Any(Any::Number(1f64))),
    //     ]);
    // let mut map = HashMap::from([
    //     ("id".to_string(), Any::String(self.id.as_str().into())),
    //     (
    //         "flavour".to_string(),
    //         Any::String(self.flavour.as_str().into()),
    //     ),
    //     ("version".to_string(), Any::Number(self.version as f64)),
    //     (
    //         "children".to_string(),
    //         Any::Array(
    //             self.children
    //                 .iter()
    //                 .map(|b| b.to_any())
    //                 .collect::<Vec<_>>()
    //                 .into(),
    //         ), // Fix: Convert tags into Prelim type
    //     ),
    //     ("title".to_string(), title),
    // ]);

    // self.props.iter().for_each(|prop| {
    //     map.insert(prop.0.clone(), prop.1.clone());
    // });

    // let map = MapPrelim::from([("a", "bb"), ("q", title)]);
    // }
}
