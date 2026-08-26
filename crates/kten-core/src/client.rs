use std::{fs, sync::Arc, time::Duration};

use reqwest::{Certificate, Client, StatusCode};
use tokio::sync::Mutex;
use url::Url;

use crate::{
    Error, Result,
    config::EffectiveConfig,
    limits::{LimitKind, Limits},
    models::{
        AddCardMemberRequest, Board, BoardStructure, Card, CardContext, CardMember, Column,
        Comment, CreateCardRequest, CurrentUser, Lane, MineCardsFilters, SearchFilters, Space,
        UpdateCardDescriptionRequest,
    },
};

#[derive(Debug, Clone)]
pub struct KaitenClientConfig {
    pub hostname: String,
    pub token: String,
    pub ca_bundle: Option<String>,
}

impl TryFrom<&EffectiveConfig> for KaitenClientConfig {
    type Error = Error;

    fn try_from(value: &EffectiveConfig) -> Result<Self> {
        Ok(Self {
            hostname: value.hostname.clone(),
            token: value.bearer_token()?.to_string(),
            ca_bundle: value.ca_bundle.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct KaitenClient {
    http: Client,
    config: KaitenClientConfig,
    throttle: Arc<Mutex<()>>,
}

impl KaitenClient {
    pub fn new(config: KaitenClientConfig) -> Result<Self> {
        Ok(Self {
            http: build_http_client(config.ca_bundle.as_deref())?,
            config,
            throttle: Arc::new(Mutex::new(())),
        })
    }

    pub async fn validate_auth(&self) -> Result<()> {
        self.get_empty("users/current").await
    }

    pub async fn current_user(&self) -> Result<CurrentUser> {
        self.get_json("users/current", &[]).await
    }

    pub async fn card(&self, id: u64) -> Result<Card> {
        self.get_json(&format!("cards/{id}"), &[]).await
    }

    pub async fn create_card(&self, request: &CreateCardRequest) -> Result<Card> {
        self.post_json("cards", request).await
    }

    pub async fn update_card_description(
        &self,
        card_id: u64,
        description: Option<String>,
    ) -> Result<Card> {
        self.patch_json(
            &format!("cards/{card_id}"),
            &UpdateCardDescriptionRequest { description },
        )
        .await
    }

    pub async fn add_card_member(&self, card_id: u64, user_id: u64) -> Result<CardMember> {
        self.post_json(
            &format!("cards/{card_id}/members"),
            &AddCardMemberRequest { user_id },
        )
        .await
    }

    pub async fn comments(&self, card_id: u64, limit: u32) -> Result<Vec<Comment>> {
        let limit = Limits::validate(Some(limit), LimitKind::Comments)?;
        self.get_json(
            &format!("cards/{card_id}/comments"),
            &[("limit", limit.to_string())],
        )
        .await
    }

    pub async fn card_context(&self, card_id: u64, comments_limit: u32) -> Result<CardContext> {
        let card = self.card(card_id).await?;
        let comments = self.comments(card_id, comments_limit).await?;
        let url = format!("https://{}/{card_id}", self.config.hostname);
        Ok(CardContext {
            card,
            comments,
            url,
        })
    }

    pub async fn search_cards(&self, filters: SearchFilters) -> Result<Vec<Card>> {
        let limit = Limits::validate(Some(filters.limit), LimitKind::Search)?;
        let mut params = vec![
            ("query", filters.query),
            ("limit", limit.to_string()),
            ("additional_card_fields", "description".to_string()),
        ];
        if let Some(space_id) = filters.space_id {
            params.push(("space_id", space_id.to_string()));
        }
        if let Some(board_id) = filters.board_id {
            params.push(("board_id", board_id.to_string()));
        }
        self.get_json("cards", &params).await
    }

    pub async fn cards_mine(&self, filters: MineCardsFilters) -> Result<Vec<Card>> {
        let current_user = self.current_user().await?;
        let limit = Limits::validate(Some(filters.limit), LimitKind::Search)?;
        let states = if filters.include_done { "1,2,3" } else { "1,2" };
        let mut params = vec![
            ("responsible_ids", current_user.id.to_string()),
            ("states", states.to_string()),
            ("condition", "1".to_string()),
            ("limit", limit.to_string()),
            ("offset", filters.offset.to_string()),
            ("additional_card_fields", "description".to_string()),
        ];
        if !filters.include_archived {
            params.push(("archived", "false".to_string()));
        }
        self.get_json("cards", &params).await
    }

    pub async fn spaces(&self, limit: u32) -> Result<Vec<Space>> {
        let limit = Limits::validate(Some(limit), LimitKind::List)?;
        self.get_json("spaces", &[("limit", limit.to_string())])
            .await
    }

    pub async fn space(&self, id: u64) -> Result<Space> {
        self.get_json(&format!("spaces/{id}"), &[]).await
    }

    pub async fn boards(&self, space_id: u64, limit: u32) -> Result<Vec<Board>> {
        let limit = Limits::validate(Some(limit), LimitKind::List)?;
        let mut boards: Vec<Board> = self
            .get_json(&format!("spaces/{space_id}/boards"), &[])
            .await?;
        boards.truncate(limit as usize);
        Ok(boards)
    }

    pub async fn board(&self, id: u64) -> Result<Board> {
        self.get_json(&format!("boards/{id}"), &[]).await
    }

    pub async fn columns(&self, board_id: u64) -> Result<Vec<Column>> {
        self.get_json(
            &format!("boards/{board_id}/columns"),
            &[("condition", "1".to_string())],
        )
        .await
    }

    pub async fn lanes(&self, board_id: u64) -> Result<Vec<Lane>> {
        self.get_json(
            &format!("boards/{board_id}/lanes"),
            &[("condition", "1".to_string())],
        )
        .await
    }

    pub async fn board_structure(&self, board_id: u64) -> Result<BoardStructure> {
        let board = self.board(board_id).await?;
        let columns = self.columns(board_id).await?;
        let lanes = self.lanes(board_id).await?;
        Ok(BoardStructure {
            board,
            columns,
            lanes,
        })
    }

    async fn get_empty(&self, path: &str) -> Result<()> {
        let response = self.send_get(path, &[]).await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(api_error(response).await)
        }
    }

    async fn get_json<T>(&self, path: &str, params: &[(&str, String)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self.send_get(path, params).await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(api_error(response).await)
        }
    }

    async fn post_json<B, T>(&self, path: &str, body: &B) -> Result<T>
    where
        B: serde::Serialize + ?Sized,
        T: serde::de::DeserializeOwned,
    {
        let url = self.url(path, &[])?;
        let _permit = self.throttle.lock().await;
        tokio::time::sleep(Duration::from_millis(210)).await;
        let response = self
            .http
            .post(url)
            .bearer_auth(&self.config.token)
            .json(body)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(api_error(response).await)
        }
    }

    async fn patch_json<B, T>(&self, path: &str, body: &B) -> Result<T>
    where
        B: serde::Serialize + ?Sized,
        T: serde::de::DeserializeOwned,
    {
        let url = self.url(path, &[])?;
        let _permit = self.throttle.lock().await;
        tokio::time::sleep(Duration::from_millis(210)).await;
        let response = self
            .http
            .patch(url)
            .bearer_auth(&self.config.token)
            .json(body)
            .send()
            .await?;
        if response.status().is_success() {
            Ok(response.json().await?)
        } else {
            Err(api_error(response).await)
        }
    }

    async fn send_get(&self, path: &str, params: &[(&str, String)]) -> Result<reqwest::Response> {
        let url = self.url(path, params)?;
        let mut retry_delay = Duration::from_millis(250);
        for attempt in 0..=3 {
            let permit = self.throttle.lock().await;
            tokio::time::sleep(Duration::from_millis(210)).await;
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.config.token)
                .send()
                .await?;
            if response.status() != StatusCode::TOO_MANY_REQUESTS || attempt == 3 {
                return Ok(response);
            }
            drop(permit);
            tokio::time::sleep(retry_delay).await;
            retry_delay *= 2;
        }
        unreachable!("retry loop always returns")
    }

    fn url(&self, path: &str, params: &[(&str, String)]) -> Result<Url> {
        let base = std::env::var("KTEN_TEST_API_BASE")
            .ok()
            .unwrap_or_else(|| format!("https://{}/api/latest", self.config.hostname));
        let base = base.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        let mut url = Url::parse(&format!("{base}/{path}"))?;
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in params {
                pairs.append_pair(key, value);
            }
        }
        Ok(url)
    }
}

fn build_http_client(ca_bundle_path: Option<&str>) -> Result<Client> {
    let mut builder = Client::builder();
    if let Some(path) = ca_bundle_path {
        for cert in load_ca_bundle(path)? {
            builder = builder.add_root_certificate(cert);
        }
    }
    Ok(builder.build()?)
}

fn load_ca_bundle(path: &str) -> Result<Vec<Certificate>> {
    let bytes = fs::read(path).map_err(|source| Error::CaBundleRead {
        path: path.to_string(),
        source,
    })?;
    if bytes.is_empty() {
        return Err(Error::CaBundleEmpty {
            path: path.to_string(),
        });
    }

    if looks_like_pem(&bytes) {
        return Certificate::from_pem_bundle(&bytes)
            .map_err(|source| Error::CaBundleParse {
                path: path.to_string(),
                details: source.to_string(),
            })
            .and_then(|bundle| {
                if bundle.is_empty() {
                    Err(Error::CaBundleParse {
                        path: path.to_string(),
                        details: "no certificates found in PEM bundle".to_string(),
                    })
                } else {
                    Ok(bundle)
                }
            });
    }

    if let Ok(bundle) = Certificate::from_pem_bundle(&bytes)
        && !bundle.is_empty()
    {
        return Ok(bundle);
    }
    Certificate::from_der(&bytes)
        .map(|cert| vec![cert])
        .map_err(|source| Error::CaBundleParse {
            path: path.to_string(),
            details: source.to_string(),
        })
}

fn looks_like_pem(bytes: &[u8]) -> bool {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.contains("-----BEGIN CERTIFICATE-----");
    }
    false
}

async fn api_error(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    let message = response
        .text()
        .await
        .unwrap_or_else(|_| "failed to read response body".to_string());
    Error::Api { status, message }
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::sync::{Mutex, OnceLock};

    use httpmock::prelude::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn client(server: &MockServer) -> KaitenClient {
        unsafe {
            std::env::set_var("KTEN_TEST_API_BASE", server.url(""));
        }
        KaitenClient::new(KaitenClientConfig {
            hostname: "company.kaiten.ru".to_string(),
            token: "token".to_string(),
            ca_bundle: None,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn handles_not_found() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/cards/1");
            then.status(404).body("not found");
        });
        let err = client(&server).card(1).await.unwrap_err();
        assert!(matches!(err, Error::Api { status: 404, .. }));
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn create_card_does_not_retry_rate_limit_response() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/cards");
            then.status(429).body("rate limited");
        });
        let err = client(&server)
            .create_card(&CreateCardRequest {
                title: "Fix login".to_string(),
                board_id: 2,
                description: None,
                column_id: None,
                lane_id: None,
                owner_id: None,
                responsible_id: None,
                due_date: None,
                due_date_time_present: None,
                asap: None,
                position: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Api { status: 429, .. }));
        assert_eq!(mock.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn update_card_description_patches_text_or_null() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        let text_mock = server.mock(|when, then| {
            when.method(PATCH)
                .path("/cards/7")
                .json_body_obj(&serde_json::json!({"description": "Updated"}));
            then.status(200)
                .json_body_obj(&serde_json::json!({"id": 7, "description": "Updated"}));
        });
        let card = client(&server)
            .update_card_description(7, Some("Updated".to_string()))
            .await
            .unwrap();
        assert_eq!(card.description.as_deref(), Some("Updated"));
        assert_eq!(text_mock.calls(), 1);

        let clear_mock = server.mock(|when, then| {
            when.method(PATCH)
                .path("/cards/8")
                .json_body_obj(&serde_json::json!({"description": null}));
            then.status(200)
                .json_body_obj(&serde_json::json!({"id": 8, "description": null}));
        });
        let card = client(&server)
            .update_card_description(8, None)
            .await
            .unwrap();
        assert_eq!(card.description, None);
        assert_eq!(clear_mock.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn handles_unauthorized_and_forbidden() {
        let _guard = env_lock().lock().unwrap();
        for status in [401, 403] {
            let server = MockServer::start();
            server.mock(|when, then| {
                when.method(GET).path("/users/current");
                then.status(status).body("auth failed");
            });
            let err = client(&server).validate_auth().await.unwrap_err();
            assert!(matches!(err, Error::Api { status: actual, .. } if actual == status));
        }
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn search_sends_filters_and_description_field_request() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/cards")
                .query_param("query", "release")
                .query_param("space_id", "10")
                .query_param("board_id", "20")
                .query_param("limit", "5")
                .query_param("additional_card_fields", "description");
            then.status(200).json_body_obj(&vec![Card {
                id: 1,
                title: Some("Release".to_string()),
                description: Some("snippet".to_string()),
                archived: None,
                state: None,
                responsible_id: None,
                owner_id: None,
                board_id: Some(20),
                column_id: None,
                lane_id: None,
            }]);
        });

        let cards = client(&server)
            .search_cards(SearchFilters {
                query: "release".to_string(),
                space_id: Some(10),
                board_id: Some(20),
                limit: 5,
            })
            .await
            .unwrap();

        assert_eq!(cards.len(), 1);
        assert_eq!(mock.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn board_structure_fetches_board_columns_and_lanes() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        let board = server.mock(|when, then| {
            when.method(GET).path("/boards/55843");
            then.status(200).json_body_obj(&serde_json::json!({
                "id": 55843,
                "title": "Development",
                "space_id": 16575
            }));
        });
        let columns = server.mock(|when, then| {
            when.method(GET)
                .path("/boards/55843/columns")
                .query_param("condition", "1");
            then.status(200).json_body_obj(&serde_json::json!([
                {"id": 189151, "title": "Backlog", "type": 1},
                {"id": 189152, "title": "In Progress", "type": 2},
                {"id": 189153, "title": "Done", "type": 3}
            ]));
        });
        let lanes = server.mock(|when, then| {
            when.method(GET)
                .path("/boards/55843/lanes")
                .query_param("condition", "1");
            then.status(200).json_body_obj(&serde_json::json!([
                {"id": 115255, "title": "Main"}
            ]));
        });

        let structure = client(&server).board_structure(55843).await.unwrap();

        assert_eq!(structure.board.id, 55843);
        assert_eq!(structure.columns.len(), 3);
        assert_eq!(structure.columns[1].title.as_deref(), Some("In Progress"));
        assert_eq!(structure.columns[1].column_type, Some(2));
        assert_eq!(structure.lanes.len(), 1);
        assert_eq!(structure.lanes[0].title.as_deref(), Some("Main"));
        assert_eq!(board.calls(), 1);
        assert_eq!(columns.calls(), 1);
        assert_eq!(lanes.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn board_columns_and_lanes_use_read_only_endpoints() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        let columns = server.mock(|when, then| {
            when.method(GET)
                .path("/boards/55843/columns")
                .query_param("condition", "1");
            then.status(200).json_body_obj(&serde_json::json!([
                {"id": 189152, "title": "In Progress", "type": 2}
            ]));
        });
        let lanes = server.mock(|when, then| {
            when.method(GET)
                .path("/boards/55843/lanes")
                .query_param("condition", "1");
            then.status(200).json_body_obj(&serde_json::json!([
                {"id": 115255, "title": "Main"}
            ]));
        });

        let column_result = client(&server).columns(55843).await.unwrap();
        let lane_result = client(&server).lanes(55843).await.unwrap();

        assert_eq!(column_result[0].id, 189152);
        assert_eq!(column_result[0].column_type, Some(2));
        assert_eq!(lane_result[0].id, 115255);
        assert_eq!(columns.calls(), 1);
        assert_eq!(lanes.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn cards_mine_resolves_current_user_and_filters_active_responsible_cards() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        let user = server.mock(|when, then| {
            when.method(GET).path("/users/current");
            then.status(200).json_body_obj(&serde_json::json!({
                "id": 42,
                "full_name": "Alex Example",
                "username": "alex.example"
            }));
        });
        let cards = server.mock(|when, then| {
            when.method(GET)
                .path("/cards")
                .query_param("responsible_ids", "42")
                .query_param("states", "1,2")
                .query_param("archived", "false")
                .query_param("condition", "1")
                .query_param("limit", "20")
                .query_param("offset", "0")
                .query_param("additional_card_fields", "description");
            then.status(200).json_body_obj(&serde_json::json!([
                {
                    "id": 123,
                    "title": "Implement card mine",
                    "description": "Details",
                    "archived": false,
                    "state": 2,
                    "responsible_id": 42,
                    "board_id": 10,
                    "column_id": 20,
                    "lane_id": 30
                }
            ]));
        });

        let result = client(&server)
            .cards_mine(MineCardsFilters {
                limit: 20,
                include_done: false,
                include_archived: false,
                offset: 0,
            })
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 123);
        assert_eq!(result[0].state, Some(2));
        assert_eq!(result[0].responsible_id, Some(42));
        assert_eq!(user.calls(), 1);
        assert_eq!(cards.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn cards_mine_can_include_done_and_archived_cards() {
        let _guard = env_lock().lock().unwrap();
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/users/current");
            then.status(200)
                .json_body_obj(&serde_json::json!({"id": 42}));
        });
        let cards = server.mock(|when, then| {
            when.method(GET)
                .path("/cards")
                .query_param("responsible_ids", "42")
                .query_param("states", "1,2,3")
                .query_param("condition", "1")
                .query_param("limit", "5")
                .query_param("offset", "10")
                .query_param("additional_card_fields", "description")
                .query_param_missing("archived");
            then.status(200).json_body_obj(&serde_json::json!([]));
        });

        let result = client(&server)
            .cards_mine(MineCardsFilters {
                limit: 5,
                include_done: true,
                include_archived: true,
                offset: 10,
            })
            .await
            .unwrap();

        assert!(result.is_empty());
        assert_eq!(cards.calls(), 1);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[tokio::test]
    async fn retries_429() {
        let _guard = env_lock().lock().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let server_calls = Arc::clone(&calls);

        tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 2048];
                let _ = stream.read(&mut buffer).await.unwrap();
                let call = server_calls.fetch_add(1, Ordering::SeqCst);
                let response = if call == 0 {
                    "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 9\r\n\r\nslow down"
                        .to_string()
                } else {
                    let body = r#"[{"id":1,"title":"Main"}]"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                stream.write_all(response.as_bytes()).await.unwrap();
            }
        });

        let client = KaitenClient::new(KaitenClientConfig {
            hostname: "company.kaiten.ru".to_string(),
            token: "token".to_string(),
            ca_bundle: None,
        })
        .unwrap();
        let base = format!("http://{addr}");
        unsafe {
            std::env::set_var("KTEN_TEST_API_BASE", base);
        }
        let spaces = client.spaces(20).await.unwrap();
        assert_eq!(spaces.len(), 1);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        unsafe {
            std::env::remove_var("KTEN_TEST_API_BASE");
        }
    }

    #[test]
    fn accepts_valid_pem_ca_bundle() {
        let cert_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("ca.pem");
        let certs = load_ca_bundle(cert_path.to_str().unwrap()).unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn rejects_missing_ca_bundle() {
        let err = load_ca_bundle("/definitely/missing/ca.pem").unwrap_err();
        assert!(matches!(err, Error::CaBundleRead { .. }));
    }

    #[test]
    fn rejects_empty_ca_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.pem");
        fs::write(&path, "").unwrap();
        let err = load_ca_bundle(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, Error::CaBundleEmpty { .. }));
    }

    #[test]
    fn rejects_invalid_ca_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("invalid.pem");
        fs::write(
            &path,
            "-----BEGIN CERTIFICATE-----\n***\n-----END CERTIFICATE-----\n",
        )
        .unwrap();
        let err = load_ca_bundle(path.to_str().unwrap()).unwrap_err();
        assert!(matches!(err, Error::CaBundleParse { .. }));
    }
}
