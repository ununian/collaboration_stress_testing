use std::vec;
use yrs::{
    encoding::{
        read::{Cursor, Read},
        write::Write,
    },
    updates::{decoder::Decode, encoder::Encode},
    Doc, ReadTxn, StateVector, Transact, Update,
};

pub trait MessageEncode {
    fn encode(&self, doc: &Doc) -> Vec<u8>;
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

impl AuthenticationMessage {
    pub fn encode(&self) -> Vec<u8> {
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
}

impl MessageEncode for SyncStepOneMessage {
    fn encode(&self, doc: &Doc) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_string(&self.document_code);
        buf.write_var(0); // message type Sync

        buf.write_var(0); // sync step 1

        let sv = doc.transact().state_vector().encode_v1();
        buf.write_buf(sv);

        buf.to_vec()
    }
}

pub struct SyncStepTwoMessage {
    pub document_code: String,
    pub remote_state: Vec<u8>,
}

impl MessageEncode for SyncStepTwoMessage {
    fn encode(&self, doc: &Doc) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_string(&self.document_code);
        buf.write_var(0); // message type Sync

        buf.write_var(1); // sync step 2
        let state_vector = StateVector::decode_v1(&self.remote_state).unwrap();

        let diff = doc.transact().encode_state_as_update_v1(&state_vector);
        buf.write_buf(&diff);

        buf.to_vec()
    }
}

pub struct UpdateMessage {
    pub document_code: String,
    pub update: Vec<u8>,
}

impl UpdateMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = vec![];
        buf.write_string(&self.document_code);
        buf.write_var(0); // message type Sync

        buf.write_var(2); // sync step 2
        buf.write_buf(&self.update);
        buf.to_vec()
    }
}

#[derive(Debug)]
pub struct IncomingMessage {
    pub document_code: String,
    pub message_type: MessageType,
}

impl IncomingMessage {
    pub fn decode(data: Vec<u8>) -> Self {
        let mut cur = Cursor::new(&data);
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
        }
    }
}

impl MessageType {
    pub fn handle_message(&self, document_code: &String, doc: &Doc, _tag: &str) -> Option<Vec<u8>> {
        match self {
            MessageType::Auth(Ok(_)) => {
                let msg = SyncStepOneMessage {
                    document_code: document_code.clone(),
                };

                Some(msg.encode(doc))
            }
            MessageType::Sync(step) => match step {
                SyncStep::One(sv) => {
                    let msg = SyncStepTwoMessage {
                        document_code: document_code.clone(),
                        remote_state: sv.clone(),
                    };

                    Some(msg.encode(doc))
                }
                SyncStep::Two(update) => {
                    let mut transact = doc.transact_mut();
                    transact.apply_update(Update::decode_v1(&update).unwrap());
                    println!("Step2 接收到 Diff ，更新文档");
                    None
                }
                SyncStep::Update(update) => {
                    let mut transact = doc.transact_mut();
                    transact.apply_update(Update::decode_v1(&update).unwrap());
                    println!("接收到 Diff ，更新文档");
                    None
                }
            },
            _ => None,
        }
    }
}
