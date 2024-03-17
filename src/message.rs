use std::vec;

use futures_util::{stream::SplitSink, SinkExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};
use yrs::{
    encoding::{
        read::{Cursor, Read},
        write::Write,
    },
    types::ToJson,
    updates::{decoder::Decode, encoder::Encode},
    Doc, GetString, Map, MapRef, ReadTxn, StateVector, Text, TextRef, Transact, Update,
};

use crate::{Client, DocInfo};

pub trait MessageEncode {
    fn encode(&self) -> Vec<u8>;
}

#[derive(Debug, Clone)]
pub enum SyncStep {
    One(Vec<u8>),
    Two(Vec<u8>),
    Update(Vec<u8>),
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Sync(SyncStep),
    Awareness,
    Auth(Result<String, String>),
    QueryAwareness,
    Stateless,
    CLOSE,
    SyncStatus,
}

pub trait MessageDecode<'a> {
    fn decode(data: &'a [u8]) -> Self;
}

pub struct AuthenticationMessage {
    pub token: String,
    pub document_code: String,
}

impl MessageEncode for AuthenticationMessage {
    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_string(&self.document_code);
        buf.write_var(2); // message type Auth

        buf.write_var(0); // token
        buf.write_string(&self.token);

        buf.to_vec()
    }
}

pub struct SyncStepOneMessage {
    pub document_code: String,
    pub doc: Doc,
}

impl MessageEncode for SyncStepOneMessage {
    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_string(&self.document_code);
        buf.write_var(0); // message type Sync

        buf.write_var(0); // sync step 1

        let sv = &self.doc.transact().state_vector().encode_v1();
        buf.write_buf(sv);

        buf.to_vec()
    }
}

pub struct UpdateMessage {
    pub document_code: String,
    pub update: Vec<u8>,
}

impl MessageEncode for UpdateMessage {
    fn encode(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_string(&self.document_code);
        buf.write_var(0); // message type Sync

        buf.write_var(2); // sync step 2
        buf.write_buf(&self.update);
        buf.to_vec()
    }
}

pub struct IncomingMessage<'a> {
    pub document_code: String,
    pub message_type: MessageType,
    pub cur: Cursor<'a>,
}

impl<'a> MessageDecode<'a> for IncomingMessage<'a> {
    fn decode(data: &'a [u8]) -> Self {
        let mut cur: Cursor<'a> = Cursor::new(data);
        let document_code = cur.read_string().unwrap().to_string();
        let message_type = cur.read_var::<u8>().unwrap();

        IncomingMessage {
            document_code,
            message_type: match message_type {
                0 => {
                    let step = cur.read_var::<u8>().unwrap();
                    MessageType::Sync(match step {
                        0 => SyncStep::One(cur.read_buf().unwrap().to_vec()),
                        1 => SyncStep::Two(cur.read_buf().unwrap().to_vec()),
                        2 => SyncStep::Update(cur.read_buf().unwrap().to_vec()),
                        _ => panic!("Unknown sync step {}", step),
                    })
                }
                1 => MessageType::Awareness,
                2 => {
                    let code = cur.read_var::<u8>().unwrap();
                    let scope = cur.read_string().unwrap().to_string();
                    MessageType::Auth(if code == 2 { Ok(scope) } else { Err(scope) })
                }
                3 => MessageType::QueryAwareness,
                5 => MessageType::Stateless,
                7 => MessageType::CLOSE,
                8 => MessageType::SyncStatus,
                _ => panic!("Unknown message type"),
            },
            cur,
        }
    }
}

pub fn next_step(msg: &MessageType, info: &DocInfo, client: &mut Client) -> Option<Vec<u8>> {
    match msg {
        MessageType::Auth(Ok(scope)) => {
            let doc = &client.doc;
            let msg = SyncStepOneMessage {
                document_code: info.document_code.clone(),
                doc: doc.clone(),
            };

            Some(msg.encode())
        }
        MessageType::Sync(step) => match step {
            SyncStep::One(_) => None,
            SyncStep::Two(update) => {
                let mut transact = client.doc.transact_mut();
                transact.apply_update(Update::decode_v1(&update).unwrap());

                Some(vec![])
            }
            SyncStep::Update(_) => None,
        },

        _ => None,
    }
}

pub fn insert_content_message(info: &DocInfo, client: &mut Client) -> Result<Vec<u8>, String> {
    let blocks = client.doc.get_or_insert_map("blocks");
    let mut txn = client.doc.transact_mut();
    let state = txn.state_vector();
    let target_block = blocks.get(&txn, &info.target_block_id);
    if target_block.is_none() {
        return Err("Target block not found".to_string());
    }
    let target_block = target_block.unwrap().cast::<MapRef>().unwrap();

    let text_prop = target_block
        .get(&txn, "prop:text")
        .unwrap()
        .cast::<TextRef>()
        .unwrap();

    text_prop.insert(&mut txn, 0, &info.content);

    txn.commit();

    let diff = txn.encode_diff_v1(&state);

    Ok(UpdateMessage {
        document_code: info.document_code.clone(),
        update: diff,
    }
    .encode())
}
