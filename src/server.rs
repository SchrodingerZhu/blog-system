#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(tag="request_type", content="request_body")]
pub enum JsonRequest {
    ListPosts
}