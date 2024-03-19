use std::{cell::RefCell, rc::Rc};

use crate::id::next_id;
use yrs::{block::Prelim, Any, Doc, Map, MapPrelim, MapRef, Transact, Transaction, TransactionMut};

pub struct BlockBuilder<'txn> {
    id: String,
    map: MapRef,
    txn: Rc<RefCell<TransactionMut<'txn>>>,
}

pub fn insert_block<'txn>(
    txn: Rc<RefCell<TransactionMut<'txn>>>,
    parent: &MapRef,
) -> BlockBuilder<'txn> {
    BlockBuilder::create(txn, parent)
}

impl<'txn> BlockBuilder<'txn> {
    pub fn create(txn: Rc<RefCell<TransactionMut<'txn>>>, parent: &MapRef) -> Self {
        let id = next_id();
        let map = parent.insert(
            &mut *txn.borrow_mut(),
            id.as_str(),
            MapPrelim::from([
                ("sys:id".to_string(), Any::String(id.as_str().into())),
                ("sys:version".to_string(), Any::Number(1f64)),
            ]),
        );

        BlockBuilder { map, txn, id }
    }

    pub fn flavour(self, flavour: &str) -> Self {
        self.map.insert(
            &mut *self.txn.borrow_mut(),
            "sys:flavour",
            Any::String(flavour.into()),
        );
        self
    }

    pub fn props<V>(self, key: &str, value: V) -> Self
    where
        V: Prelim,
    {
        self.map
            .insert(&mut *self.txn.borrow_mut(), format!("props:{}", key), value);
        self
    }

    pub fn children(self, children: Vec<String>) -> Self {
        self.map.insert(
            &mut *self.txn.borrow_mut(),
            "sys:children",
            Any::Array(
                children
                    .iter()
                    .map(|id| Any::String(id.as_str().into()))
                    .collect(),
            ),
        );
        self
    }

    pub fn id(self) -> String {
        self.id
    }
}
