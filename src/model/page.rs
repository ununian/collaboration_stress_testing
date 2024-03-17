use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde_json::json;
use yrs::{
    types::ToJson, Any, Doc, Map, MapRef, ReadTxn, StateVector, TextPrelim, Transact,
    TransactionMut, Value,
};

use crate::util::with_uuid_option;

use super::{
    block::{insert_block, BlockBuilder},
    r#const::*,
};

pub struct Page {
    pub root: Doc,
}

impl Page {
    fn create_empty_page_doc(document_name: &str) -> Page {
        let id: String = format!(
            "{}::{}::{}",
            PAGE_PREFIX, document_name, WORKSPACE_DEFAULT_PAGE_NAME
        );
        let doc = Doc::with_options(with_uuid_option(&id));
        let blocks_map = doc.get_or_insert_map("blocks");

        {
            let txn = Rc::new(RefCell::new(doc.transact_mut()));

            let paragraph = insert_block(txn.clone(), &blocks_map)
                .flavour("wq:paragraph")
                .props("text", TextPrelim::new(""))
                .children(vec![])
                .id();

            let note = insert_block(txn.clone(), &blocks_map)
                .flavour("wq::note")
                .props("xywh", "[0,0,800,95]")
                .props("background", "--wq-background-secondary-color")
                .props("index", "a0")
                .props("hidden", false)
                .props("displayMode", "both")
                .props(
                    "edgeless",
                    Any::Map(
                        HashMap::from([(
                            "style".to_string(),
                            Any::Map(
                                HashMap::from([
                                    ("borderRadius".to_string(), Any::Number(8f64)),
                                    ("borderSize".to_string(), Any::Number(4f64)),
                                    ("borderStyle".to_string(), Any::String("solid".into())),
                                    (
                                        "shadowType".to_string(),
                                        Any::String("--wq-note-shadow-box".into()),
                                    ),
                                ])
                                .into(),
                            ),
                        )])
                        .into(),
                    ),
                )
                .children(vec![paragraph])
                .id();

            insert_block(txn.clone(), &blocks_map)
                .flavour("wq::page")
                .props("title", TextPrelim::new(""))
                .children(vec![note])
                .id();
        }

        Page { root: doc }
    }

    fn to_json(&self) -> Any {
        self.root.to_json(&self.root.transact())
    }

    fn get_blocks(&self) -> MapRef {
        self.root.get_or_insert_map("blocks")
    }

    fn get_transact_ref(&self) -> Rc<RefCell<TransactionMut<'_>>> {
        Rc::new(RefCell::new(self.root.transact_mut()))
    }

    fn insert_block<F>(&self, func: F) -> String
    where
        F: Fn(Rc<RefCell<TransactionMut<'_>>>, &MapRef) -> String,
    {
        let blocks = self.get_blocks();
        func(self.get_transact_ref(), &blocks)
    }

    fn encode(&self) -> Vec<u8> {
        self.root
            .transact()
            .encode_state_as_update_v1(&StateVector::default())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use yrs::Any;

    use crate::{model::block::insert_block, util::to_hex_string};

    use super::Page;

    #[test]
    fn test_create() -> Result<()> {
        let empty_page = Page::create_empty_page_doc("test");
        println!("{:#?}", empty_page.to_json());

        empty_page.insert_block(|txn, blocks| {
            insert_block(txn, blocks)
                .flavour("wq:columns")
                .props("key", Any::String("value".into()))
                .id()
        });
        Ok(())
    }

    #[test]
    fn test_encode() -> Result<()> {
        let empty_page = Page::create_empty_page_doc("test");
        // println!("{}", to_hex_string(&empty_page.encode()));
        let mut str = String::new();
        empty_page.to_json().to_json(&mut str);
        println!("{}", str);
        Ok(())
    }
}
