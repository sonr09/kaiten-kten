use kten_core::{
    KaitenClient, KaitenClientConfig,
    limits::{LimitKind, Limits},
    models::{CreateCardRequest, SearchFilters},
    render,
};
use rmcp::{
    ErrorData as McpError, ServiceExt, handler::server::wrapper::Parameters, schemars, tool,
    tool_router, transport::stdio,
};
use serde::Deserialize;

#[derive(Clone)]
pub struct KtenMcp {
    client: KaitenClient,
}

impl KtenMcp {
    pub fn new(config: KaitenClientConfig) -> anyhow::Result<Self> {
        Ok(Self {
            client: KaitenClient::new(config)?,
        })
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CardParams {
    pub id: u64,
    pub comments_limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CommentsParams {
    pub id: u64,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    pub query: String,
    pub space_id: Option<u64>,
    pub board_id: Option<u64>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListParams {
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct IdParams {
    pub id: u64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BoardListParams {
    pub space_id: u64,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum CreatePosition {
    First,
    Last,
}

impl CreatePosition {
    fn api_value(self) -> u8 {
        match self {
            Self::First => 1,
            Self::Last => 2,
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateCardParams {
    /// Card title. Kaiten accepts 1-1024 characters.
    pub title: String,
    /// Destination board ID.
    pub board_id: u64,
    /// Card description.
    pub description: Option<String>,
    /// Destination column ID.
    pub column_id: Option<u64>,
    /// Destination lane ID.
    pub lane_id: Option<u64>,
    /// Owner user ID.
    pub owner_id: Option<u64>,
    /// Responsible user ID.
    pub responsible_id: Option<u64>,
    /// Deadline in ISO 8601 format.
    pub due_date: Option<String>,
    /// Whether the deadline includes hours and minutes.
    pub due_date_time_present: Option<bool>,
    /// Whether to mark the card as ASAP.
    pub asap: Option<bool>,
    /// Place the card first or last in its board cell.
    pub position: Option<CreatePosition>,
}

#[tool_router(server_handler)]
impl KtenMcp {
    #[tool(
        description = "Create a Kaiten card and return it as structured JSON. This changes Kaiten state and is not idempotent.",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn kten_create_card(
        &self,
        Parameters(params): Parameters<CreateCardParams>,
    ) -> Result<String, McpError> {
        let card = self
            .client
            .create_card(&CreateCardRequest {
                title: params.title,
                board_id: params.board_id,
                description: params.description,
                column_id: params.column_id,
                lane_id: params.lane_id,
                owner_id: params.owner_id,
                responsible_id: params.responsible_id,
                due_date: params.due_date,
                due_date_time_present: params.due_date_time_present,
                asap: params.asap,
                position: params.position.map(CreatePosition::api_value),
            })
            .await
            .map_err(to_mcp)?;
        render::json(&card).map_err(to_mcp)
    }

    #[tool(description = "Get an LLM-ready markdown context for a Kaiten card")]
    async fn kten_get_card_context(
        &self,
        Parameters(params): Parameters<CardParams>,
    ) -> Result<String, McpError> {
        let limit = Limits::validate(params.comments_limit, LimitKind::Comments).map_err(to_mcp)?;
        let context = self
            .client
            .card_context(params.id, limit)
            .await
            .map_err(to_mcp)?;
        Ok(render::context_markdown(&context))
    }

    #[tool(description = "Search Kaiten cards and return structured JSON")]
    async fn kten_search_cards(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<String, McpError> {
        let limit = Limits::validate(params.limit, LimitKind::Search).map_err(to_mcp)?;
        let cards = self
            .client
            .search_cards(SearchFilters {
                query: params.query,
                space_id: params.space_id,
                board_id: params.board_id,
                limit,
            })
            .await
            .map_err(to_mcp)?;
        render::json(&cards).map_err(to_mcp)
    }

    #[tool(description = "Get Kaiten card comments as structured JSON")]
    async fn kten_get_comments(
        &self,
        Parameters(params): Parameters<CommentsParams>,
    ) -> Result<String, McpError> {
        let limit = Limits::validate(params.limit, LimitKind::Comments).map_err(to_mcp)?;
        let comments = self
            .client
            .comments(params.id, limit)
            .await
            .map_err(to_mcp)?;
        render::json(&comments).map_err(to_mcp)
    }

    #[tool(description = "List Kaiten spaces as structured JSON")]
    async fn kten_list_spaces(
        &self,
        Parameters(params): Parameters<ListParams>,
    ) -> Result<String, McpError> {
        let limit = Limits::validate(params.limit, LimitKind::List).map_err(to_mcp)?;
        let spaces = self.client.spaces(limit).await.map_err(to_mcp)?;
        render::json(&spaces).map_err(to_mcp)
    }

    #[tool(description = "Get a Kaiten space as structured JSON")]
    async fn kten_get_space(
        &self,
        Parameters(params): Parameters<IdParams>,
    ) -> Result<String, McpError> {
        let space = self.client.space(params.id).await.map_err(to_mcp)?;
        render::json(&space).map_err(to_mcp)
    }

    #[tool(description = "List Kaiten boards for a space as structured JSON")]
    async fn kten_list_boards(
        &self,
        Parameters(params): Parameters<BoardListParams>,
    ) -> Result<String, McpError> {
        let limit = Limits::validate(params.limit, LimitKind::List).map_err(to_mcp)?;
        let boards = self
            .client
            .boards(params.space_id, limit)
            .await
            .map_err(to_mcp)?;
        render::json(&boards).map_err(to_mcp)
    }

    #[tool(description = "Get a Kaiten board as structured JSON")]
    async fn kten_get_board(
        &self,
        Parameters(params): Parameters<IdParams>,
    ) -> Result<String, McpError> {
        let board = self.client.board(params.id).await.map_err(to_mcp)?;
        render::json(&board).map_err(to_mcp)
    }
}

pub async fn serve_stdio(config: KaitenClientConfig) -> anyhow::Result<()> {
    let service = KtenMcp::new(config)?.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

fn to_mcp(error: impl std::fmt::Display) -> McpError {
    McpError::internal_error(error.to_string(), None)
}
