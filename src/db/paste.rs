use mongodb::bson;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Paste {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub title: String,
    pub content: String,
    pub author_id: String,
}
