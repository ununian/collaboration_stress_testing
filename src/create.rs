use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResponse {
    pub success: bool,
    pub code: i64,
    pub data: Data,
    pub json: String,
    pub rsp_type: String,
    pub message: String,
    pub timestamp: i64,
    pub serializer: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub id: String,
    pub space_code: String,
    pub space_name: String,
    pub folder_id: String,
    pub folder_name: String,
    pub classify_code: String,
    pub classify_name: String,
    pub code: String,
    pub title: String,
    pub upload_status: String,
    pub file_url: String,
    pub file_suffix: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub mode_type: String,
    pub space_default_flag: String,
    pub create_time: String,
    pub create_name: String,
    pub update_time: String,
    pub mark_record: MarkRecord,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkRecord {}

pub async fn create_docs(
    num: usize,
    origin: &String,
    prefix: &String,
    token: &String,
) -> Vec<String> {
    let mut handles = Vec::new();
    let mut codes = Vec::new();

    for i in 0..num {
        let origin = Arc::new(origin.clone());
        let prefix = Arc::new(prefix.clone());
        let token = Arc::new(token.clone());
        let handle = tokio::spawn(create_doc(origin, prefix, token, i));
        handles.push(handle);
    }

    for handle in handles {
        let code = handle.await.unwrap();
        if code.is_ok() {
            codes.push(code.unwrap());
        } else {
            eprintln!("Error: 创建文档失败 {:?}", code.err());
        }
    }

    codes
}

pub async fn create_doc(
    origin: Arc<String>,
    prefix: Arc<String>,
    token: Arc<String>,
    index: usize,
) -> Result<String> {
    let client = reqwest::Client::new();
    let data = json!({"base":{"title":format!("{}_测试文档_{}",prefix,index),"type":"DWQ"}});

    let res = client
        .post(format!(
            "{}/cloudtest/workbench/v2/userResource/save",
            origin
        ))
        .header("Authorization", token.as_str())
        .json(&data)
        .send()
        .await?
        .json::<CreateResponse>()
        .await?;

    Ok(res.data.code.to_string())
}
