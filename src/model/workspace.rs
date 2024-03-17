use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use yrs::{Any, ArrayPrelim, Doc, Map, Transact};

use crate::util::with_uuid_option;

use super::r#const::{PAGE_PREFIX, WORKSPACE_DEFAULT_PAGE_NAME, WORKSPACE_PREFIX};

#[derive(Debug)]
pub struct Workspace {
    pub document_name: String,
}

#[derive(Debug, Clone)]
struct PageMeta {
    pub id: String,
    pub title: String,
    pub createDate: f64,
    pub tags: Vec<String>,
}

impl PageMeta {
    pub fn new(document_name: &String) -> PageMeta {
        PageMeta {
            id: format!(
                "{}::{}::{}",
                PAGE_PREFIX, document_name, WORKSPACE_DEFAULT_PAGE_NAME
            ),
            title: "".to_string(),
            createDate: match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(n) => n.as_millis() as f64,
                Err(_) => panic!("SystemTime before UNIX EPOCH!"),
            },
            tags: vec![],
        }
    }

    pub fn to_any(&self) -> Any {
        Any::Map(Arc::new(HashMap::from([
            ("id".to_string(), Any::String(self.id.as_str().into())),
            ("title".to_string(), Any::String(self.title.as_str().into())),
            ("createDate".to_string(), Any::Number(self.createDate)),
            (
                "tags".to_string(),
                Any::Array(vec![].into()), // Fix: Convert tags into Prelim type
            ),
        ])))
    }
}

impl Workspace {
    pub fn new(document_name: String) -> Workspace {
        Workspace { document_name }
    }

    pub fn to_yjs(&self) -> Doc {
        let guid = format!("{}::{}", WORKSPACE_PREFIX, self.document_name);
        let doc = Doc::with_options(with_uuid_option(&guid));
        let meta = doc.get_or_insert_map("meta");
        let pages = ArrayPrelim::from([PageMeta::new(&self.document_name).to_any()]);
        {
            let mut txn: yrs::TransactionMut<'_> = doc.transact_mut();
            meta.insert(&mut txn, "pages", pages); // Fix: Convert pages into Prelim type
        }

        doc
    }
}
