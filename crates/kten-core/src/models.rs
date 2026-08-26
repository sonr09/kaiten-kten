use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Card {
    pub id: u64,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub archived: Option<bool>,
    #[serde(default)]
    pub state: Option<u8>,
    #[serde(default)]
    pub responsible_id: Option<u64>,
    #[serde(default)]
    pub owner_id: Option<u64>,
    #[serde(default)]
    pub board_id: Option<u64>,
    #[serde(default)]
    pub column_id: Option<u64>,
    #[serde(default)]
    pub lane_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateCardRequest {
    pub title: String,
    pub board_id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsible_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_time_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asap: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCardDescriptionRequest {
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurrentUser {
    pub id: u64,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    pub id: u64,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub updated: Option<String>,
    #[serde(default)]
    pub author: Option<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: Option<u64>,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Space {
    pub id: u64,
    pub title: Option<String>,
    #[serde(default)]
    pub company_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Board {
    pub id: u64,
    pub title: Option<String>,
    #[serde(default)]
    pub space_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Column {
    pub id: u64,
    pub title: Option<String>,
    #[serde(default, rename = "type")]
    pub column_type: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lane {
    pub id: u64,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoardStructure {
    pub board: Board,
    pub columns: Vec<Column>,
    pub lanes: Vec<Lane>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchFilters {
    pub query: String,
    pub space_id: Option<u64>,
    pub board_id: Option<u64>,
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MineCardsFilters {
    pub limit: u32,
    pub include_done: bool,
    pub include_archived: bool,
    pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardContext {
    pub card: Card,
    pub comments: Vec<Comment>,
    pub url: String,
}
