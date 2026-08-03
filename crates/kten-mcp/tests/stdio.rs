use kten_core::KaitenClientConfig;
use kten_mcp::KtenMcp;
use rmcp::ServiceExt;

#[tokio::test]
async fn exposes_required_tools_over_sdk_transport() {
    let (client_stream, server_stream) = tokio::io::duplex(8192);
    let server = KtenMcp::new(KaitenClientConfig {
        hostname: "company.kaiten.ru".to_string(),
        token: "token".to_string(),
        ca_bundle: None,
    })
    .unwrap();

    let server_task = tokio::spawn(async move { server.serve(server_stream).await.unwrap() });
    let client = ().serve(client_stream).await.unwrap();
    let tools = client.peer().list_tools(Default::default()).await.unwrap();
    let names = tools
        .tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<Vec<_>>();

    for expected in [
        "kten_create_card",
        "kten_get_card_context",
        "kten_search_cards",
        "kten_get_comments",
        "kten_list_spaces",
        "kten_get_space",
        "kten_list_boards",
        "kten_get_board",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }

    let create = tools
        .tools
        .iter()
        .find(|tool| tool.name == "kten_create_card")
        .unwrap();
    let annotations = create.annotations.as_ref().unwrap();
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.idempotent_hint, Some(false));

    client.cancel().await.unwrap();
    let running_server = server_task.await.unwrap();
    running_server.cancel().await.unwrap();
}
