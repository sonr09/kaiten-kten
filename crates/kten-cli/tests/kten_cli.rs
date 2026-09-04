use std::fs;
use std::process::Command;

use httpmock::prelude::*;

#[test]
fn auth_login_status_and_logout_use_single_profile_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/users/current");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 1}));
    });

    let login = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "--hostname",
            "company.kaiten.ru",
            "--token",
            "secret-token",
            "auth",
            "login",
        ])
        .output()
        .unwrap();
    assert!(login.status.success(), "{}", stderr(&login));

    let status = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["auth", "status"])
        .output()
        .unwrap();
    assert!(status.status.success(), "{}", stderr(&status));
    let stdout = String::from_utf8(status.stdout).unwrap();
    assert!(stdout.contains("company.kaiten.ru"));
    assert!(stdout.contains("secr..."));
    assert!(!stdout.contains("secret-token"));

    let logout = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["auth", "logout"])
        .output()
        .unwrap();
    assert!(logout.status.success(), "{}", stderr(&logout));
}

#[test]
fn auth_login_failed_validation_does_not_write_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/users/current");
        then.status(401).body("unauthorized");
    });

    let login = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "--hostname",
            "company.kaiten.ru",
            "--token",
            "secret-token",
            "auth",
            "login",
        ])
        .output()
        .unwrap();
    assert!(!login.status.success());
    assert!(
        !config_path.exists()
            || fs::read_to_string(&config_path)
                .unwrap_or_default()
                .is_empty()
    );
}

#[test]
fn auth_status_show_token_reveals_full_token() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/users/current");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 1}));
    });

    let login = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "--hostname",
            "company.kaiten.ru",
            "--token",
            "secret-token",
            "auth",
            "login",
        ])
        .output()
        .unwrap();
    assert!(login.status.success(), "{}", stderr(&login));

    let status = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["auth", "status", "--show-token"])
        .output()
        .unwrap();
    assert!(status.status.success(), "{}", stderr(&status));
    let stdout = String::from_utf8(status.stdout).unwrap();
    assert!(stdout.contains("secret-token"));
}

#[test]
fn root_help_hides_removed_domain_and_ca_flags() {
    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--hostname"));
    assert!(stdout.contains("--token"));
    assert!(!stdout.contains("--domain"));
    assert!(!stdout.contains("--api-base"));
    assert!(!stdout.contains("--ca-bundle"));
}

#[test]
fn config_help_lists_only_get_set_edit() {
    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .args(["config", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("get"));
    assert!(stdout.contains("set"));
    assert!(stdout.contains("edit"));
    assert!(!stdout.contains("list"));
    assert!(!stdout.contains("path"));
}

#[test]
fn config_list_and_path_are_removed() {
    for args in [vec!["config", "list"], vec!["config", "path"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}

#[test]
fn config_set_rejects_secret_token_key() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["config", "set", "token", "secret-token"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("secret config key"));
}

#[test]
fn config_get_ca_bundle_respects_env_override() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let set_ca = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["config", "set", "ca_bundle", "/tmp/from-file.pem"])
        .output()
        .unwrap();
    assert!(set_ca.status.success(), "{}", stderr(&set_ca));

    let from_file = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["config", "get", "ca_bundle"])
        .output()
        .unwrap();
    assert!(from_file.status.success(), "{}", stderr(&from_file));
    assert_eq!(
        String::from_utf8(from_file.stdout).unwrap().trim(),
        "/tmp/from-file.pem"
    );

    let from_env = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_CA_BUNDLE", "/tmp/from-env.pem")
        .args(["config", "get", "ca_bundle"])
        .output()
        .unwrap();
    assert!(from_env.status.success(), "{}", stderr(&from_env));
    assert_eq!(
        String::from_utf8(from_env.stdout).unwrap().trim(),
        "/tmp/from-env.pem"
    );
}

#[test]
fn config_get_respects_effective_hostname_and_redacts_token() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/users/current");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 1}));
    });

    let login = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "--hostname",
            "file.kaiten.ru",
            "--token",
            "secret-token",
            "auth",
            "login",
        ])
        .output()
        .unwrap();
    assert!(login.status.success(), "{}", stderr(&login));

    let hostname = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("KTEN_HOSTNAME", "env.kaiten.ru")
        .args(["config", "get", "default_hostname"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(hostname.stdout).unwrap().trim(),
        "env.kaiten.ru"
    );

    let token = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["config", "get", "token"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(token.stdout).unwrap().trim(),
        "<redacted>"
    );
}

#[test]
fn config_edit_requires_editor_env() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env_remove("VISUAL")
        .env_remove("EDITOR")
        .args(["config", "edit"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("set VISUAL or EDITOR"));
}

#[test]
fn config_edit_propagates_editor_failure() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("EDITOR", "sh -c 'exit 7'")
        .args(["config", "edit"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("editor exited with status"));
}

#[test]
fn config_edit_reloads_and_fails_on_invalid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let success = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env(
            "EDITOR",
            "sh -c \"printf 'default_hostname = \\\"edited.kaiten.ru\\\"\\n' > \\\"$1\\\"\" x",
        )
        .args(["config", "edit"])
        .output()
        .unwrap();
    assert!(success.status.success(), "{}", stderr(&success));

    let get = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["config", "get", "default_hostname"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(get.stdout).unwrap().trim(),
        "edited.kaiten.ru"
    );

    let invalid = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env(
            "EDITOR",
            "sh -c \"printf 'not=valid=toml\\n' > \\\"$1\\\"\" x",
        )
        .args(["config", "edit"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert!(stderr(&invalid).contains("failed to reload config after edit"));
}

#[test]
fn card_view_json_uses_read_only_http_get() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/cards/123");
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "description": "Details",
            "ignored": "field"
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "view", "123", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"id\": 123"));
    assert!(stdout.contains("\"title\": \"Fix login\""));
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_view_shows_parent_and_child_relations_in_human_and_json() {
    let server = MockServer::start();
    let card = server.mock(|when, then| {
        when.method(GET).path("/cards/123");
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Delivery",
            "parents": [{
                "id": 100,
                "title": "Program",
                "state": 2,
                "archived": false,
                "board_id": 10,
                "column_id": 20,
                "lane_id": 30
            }],
            "children": [{
                "id": 200,
                "title": "Implementation",
                "state": 1,
                "archived": false,
                "board_id": 11,
                "column_id": 21,
                "lane_id": null
            }]
        }));
    });
    let run = |json: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kten"));
        command
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "view", "123"]);
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    };

    let human = run(false);
    assert!(human.status.success(), "{}", stderr(&human));
    let stdout = String::from_utf8(human.stdout).unwrap();
    assert!(stdout.contains("Parents:\n- #100 Program"));
    assert!(stdout.contains("Children:\n- #200 Implementation"));

    let json_output = run(true);
    assert!(json_output.status.success(), "{}", stderr(&json_output));
    let json: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(json["parents"][0]["id"], 100);
    assert_eq!(json["parents"][0]["title"], "Program");
    assert_eq!(json["children"][0]["id"], 200);
    assert_eq!(json["children"][0]["board_id"], 11);
    assert_eq!(card.calls(), 2);
}

#[test]
fn card_update_patches_description_and_supports_clearing() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({"description": "Updated"}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "description": "Updated"
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "update",
            "123",
            "--description",
            "Updated",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("\"description\": \"Updated\"")
    );
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_update_sets_and_clears_high_priority() {
    for (priority, asap) in [("high", true), ("normal", false)] {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(PATCH)
                .path("/cards/123")
                .json_body_obj(&serde_json::json!({"asap": asap}));
            then.status(200).json_body_obj(&serde_json::json!({
                "id": 123,
                "title": "Fix login",
                "asap": asap
            }));
        });

        let mut args = vec!["card", "update", "123", "--priority", priority];
        if !asap {
            args.push("--json");
        }
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(args)
            .output()
            .unwrap();

        assert!(output.status.success(), "{}", stderr(&output));
        let stdout = String::from_utf8(output.stdout).unwrap();
        if asap {
            assert!(stdout.contains("Priority: High"));
        } else {
            assert!(stdout.contains("\"asap\": false"));
        }
        assert_eq!(mock.calls(), 1);
    }
}

#[test]
fn card_update_sets_story_points_and_prints_human_output() {
    let server = MockServer::start();
    let property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Story Point")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 42,
            "name": "Story Point",
            "type": "number",
            "condition": "active"
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({
                "properties": {"id_42": 5}
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "properties": {"id_42": 5}
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "update", "123", "--story-points", "5"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Updated custom property Story Point: 5")
    );
    assert_eq!(property.calls(), 1);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_sets_task_priority_custom_property() {
    let server = MockServer::start();
    let property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Приоритет задачи")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 84,
            "name": "Приоритет задачи",
            "type": "number",
            "condition": "active"
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({
                "properties": {"id_84": 73}
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "properties": {"id_84": 73}
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "update", "123", "--task-priority", "73"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Updated custom property Приоритет задачи: 73")
    );
    assert_eq!(property.calls(), 1);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_rejects_invalid_task_priority_before_http() {
    let server = MockServer::start();
    let reads = server.mock(|when, then| {
        when.method(GET);
        then.status(500);
    });
    let updates = server.mock(|when, then| {
        when.method(PATCH);
        then.status(500);
    });

    for value in ["0", "101", "-1", "1.5", "high", ""] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "update", "123", "--task-priority", value])
            .output()
            .unwrap();

        assert!(!output.status.success(), "accepted {value:?}");
        assert!(
            stderr(&output).contains("task priority must be an integer from 1 to 100"),
            "unexpected error for {value:?}: {}",
            stderr(&output)
        );
    }

    assert_eq!(reads.calls(), 0);
    assert_eq!(updates.calls(), 0);
}

#[test]
fn card_update_sets_size_and_prints_human_output() {
    let server = MockServer::start();
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({"size_text": "5"}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "size_text": "5"
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "update", "123", "--size", "5"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Size: 5")
    );
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_sets_named_custom_property_and_prints_json() {
    let server = MockServer::start();
    let number_property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Cost of Delay")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 77,
            "name": "Cost of Delay",
            "type": "number",
            "condition": "active"
        }]));
    });
    let string_property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Release Train")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 78,
            "name": "Release Train",
            "type": "string",
            "condition": "active"
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({
                "properties": {"id_77": 8.5, "id_78": "R2"}
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "properties": {"id_77": 8.5, "id_78": "R2"}
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "update",
            "123",
            "--custom-property",
            "Cost of Delay=8.5",
            "--custom-property",
            "Release Train=\"R2\"",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["properties"]["id_77"], 8.5);
    assert_eq!(json["properties"]["id_78"], "R2");
    assert_eq!(number_property.calls(), 1);
    assert_eq!(string_property.calls(), 1);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_rejects_invalid_custom_property_before_http() {
    let server = MockServer::start();
    let requests = server.mock(|when, then| {
        when.method(GET);
        then.status(500);
    });
    let updates = server.mock(|when, then| {
        when.method(PATCH);
        then.status(500);
    });

    for value in ["missing-equals", "=1", "Name=", "Name=plain-text"] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "update", "123", "--custom-property", value])
            .output()
            .unwrap();

        assert!(!output.status.success(), "accepted {value:?}");
        assert!(
            stderr(&output).contains("custom property"),
            "unexpected error for {value:?}: {}",
            stderr(&output)
        );
    }

    assert_eq!(requests.calls(), 0);
    assert_eq!(updates.calls(), 0);
}

#[test]
fn card_update_resolves_story_points_safely_before_patch() {
    let cases = [
        (
            serde_json::json!([]),
            "expected exactly one active custom property named \"Story Point\", found 0",
        ),
        (
            serde_json::json!([
                {"id": 42, "name": "Story Point", "type": "number", "condition": "active"},
                {"id": 43, "name": "Story Point", "type": "number", "condition": "active"}
            ]),
            "expected exactly one active custom property named \"Story Point\", found 2",
        ),
        (
            serde_json::json!([
                {"id": 42, "name": "Story Point", "type": "string", "condition": "active"}
            ]),
            "custom property \"Story Point\" must have type number",
        ),
    ];

    for (properties, expected_error) in cases {
        let server = MockServer::start();
        let property = server.mock(|when, then| {
            when.method(GET)
                .path("/company/custom-properties")
                .query_param("query", "Story Point")
                .query_param("compact", "true");
            then.status(200).json_body_obj(&properties);
        });
        let update = server.mock(|when, then| {
            when.method(PATCH);
            then.status(200);
        });

        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "update", "123", "--story-points", "5"])
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(
            stderr(&output).contains(expected_error),
            "unexpected error: {}",
            stderr(&output)
        );
        assert_eq!(property.calls(), 1);
        assert_eq!(update.calls(), 0);
    }
}

#[test]
fn card_update_rejects_duplicate_custom_property_before_http() {
    let server = MockServer::start();
    let requests = server.mock(|when, then| {
        when.method(GET);
        then.status(500);
    });
    let updates = server.mock(|when, then| {
        when.method(PATCH);
        then.status(500);
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "update",
            "123",
            "--custom-property",
            "Cost=1",
            "--custom-property",
            "Cost=2",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("custom property names must not be repeated"));
    assert_eq!(requests.calls(), 0);
    assert_eq!(updates.calls(), 0);
}

#[test]
fn card_update_clears_story_points_and_prints_json() {
    let server = MockServer::start();
    let property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Story Point")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 42,
            "name": "Story Point",
            "type": "number",
            "condition": "active"
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({
                "properties": {"id_42": null}
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "properties": {"id_42": null}
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "update", "123", "--story-points", "", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["properties"]["id_42"], serde_json::Value::Null);
    assert_eq!(property.calls(), 1);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_clears_size_and_prints_json() {
    let server = MockServer::start();
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({"size_text": null}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "size_text": null
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "update", "123", "--size", "", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["size_text"], serde_json::Value::Null);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_rejects_invalid_story_points_before_http() {
    let server = MockServer::start();
    let reads = server.mock(|when, then| {
        when.method(GET);
        then.status(500);
    });
    let updates = server.mock(|when, then| {
        when.method(PATCH);
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "size_text": "unexpected"
        }));
    });

    for value in ["-1", "NaN", "inf", "not-a-number", "   "] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "update", "123", "--story-points", value])
            .output()
            .unwrap();

        assert!(!output.status.success(), "accepted {value:?}");
        assert!(
            stderr(&output).contains("story points must be a finite non-negative number"),
            "unexpected error for {value:?}: {}",
            stderr(&output)
        );
    }

    assert_eq!(reads.calls(), 0);
    assert_eq!(updates.calls(), 0);
}

#[test]
fn card_update_help_distinguishes_story_points_size_and_custom_properties() {
    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .args(["card", "update", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--story-points <NUMBER>"));
    assert!(stdout.contains("Custom 'Story Point' number"));
    assert!(stdout.contains("--task-priority <1..100>"));
    assert!(stdout.contains("custom 'Приоритет задачи' number"));
    assert!(stdout.contains("--size <NUMBER>"));
    assert!(stdout.contains("Kaiten Size"));
    assert!(stdout.contains("--custom-property <NAME=JSON>"));
    assert!(stdout.contains("repeat to update multiple properties"));
}

#[test]
fn card_update_combines_story_points_with_other_fields() {
    let server = MockServer::start();
    let property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Story Point")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 42,
            "name": "Story Point",
            "type": "number",
            "condition": "active"
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({
                "description": "Estimated",
                "asap": true,
                "size_text": "8",
                "properties": {"id_42": 3.5}
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "description": "Estimated",
            "asap": true,
            "size_text": "8",
            "properties": {"id_42": 3.5}
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "update",
            "123",
            "--description",
            "Estimated",
            "--priority",
            "high",
            "--story-points",
            "3.5",
            "--size",
            "8",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["description"], "Estimated");
    assert_eq!(json["asap"], true);
    assert_eq!(json["size_text"], "8");
    assert_eq!(json["properties"]["id_42"], 3.5);
    assert_eq!(property.calls(), 1);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_update_story_points_does_not_retry_failed_patch() {
    let server = MockServer::start();
    let property = server.mock(|when, then| {
        when.method(GET)
            .path("/company/custom-properties")
            .query_param("query", "Story Point")
            .query_param("compact", "true");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 42,
            "name": "Story Point",
            "type": "number",
            "condition": "active"
        }]));
    });
    let update = server.mock(|when, then| {
        when.method(PATCH)
            .path("/cards/123")
            .json_body_obj(&serde_json::json!({
                "properties": {"id_42": 5}
            }));
        then.status(429).body("rate limited");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "update", "123", "--story-points", "5"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("429"));
    assert_eq!(property.calls(), 1);
    assert_eq!(update.calls(), 1);
}

#[test]
fn card_member_add_posts_user_id_and_prints_member() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/members")
            .json_body_obj(&serde_json::json!({"user_id": 42}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 42,
            "full_name": "Ada Lovelace",
            "username": "ada",
            "email": "ada@example.com",
            "type": 1
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "member", "add", "123", "--user", "42", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"id\": 42"));
    assert!(stdout.contains("\"type\": 1"));
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_child_add_posts_relation_and_supports_human_and_json_output() {
    let server = MockServer::start();
    let relation = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/children")
            .json_body_obj(&serde_json::json!({"card_id": 456}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 456,
            "title": "Implementation",
            "parents": [{"id": 123, "title": "Delivery"}],
            "children": []
        }));
    });
    let run = |json: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kten"));
        command
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "child", "add", "123", "--child", "456"]);
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    };

    let human = run(false);
    assert!(human.status.success(), "{}", stderr(&human));
    assert_eq!(
        String::from_utf8(human.stdout).unwrap(),
        "Added child card #456 to parent card #123.\n"
    );

    let json_output = run(true);
    assert!(json_output.status.success(), "{}", stderr(&json_output));
    let json: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(json["id"], 456);
    assert_eq!(json["parents"][0]["id"], 123);
    assert_eq!(relation.calls(), 2);
}

#[test]
fn card_child_remove_deletes_relation_and_supports_human_and_json_output() {
    let server = MockServer::start();
    let relation = server.mock(|when, then| {
        when.method(DELETE).path("/cards/123/children/456");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 9001}));
    });
    let run = |json: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kten"));
        command
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "child", "remove", "123", "--child", "456"]);
        if json {
            command.arg("--json");
        }
        command.output().unwrap()
    };

    let human = run(false);
    assert!(human.status.success(), "{}", stderr(&human));
    assert_eq!(
        String::from_utf8(human.stdout).unwrap(),
        "Removed child card #456 from parent card #123.\n"
    );

    let json_output = run(true);
    assert!(json_output.status.success(), "{}", stderr(&json_output));
    let json: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(json["id"], 9001);
    assert_eq!(relation.calls(), 2);
}

#[test]
fn card_child_commands_reject_invalid_or_equal_ids_before_http() {
    let server = MockServer::start();
    let posts = server.mock(|when, then| {
        when.method(POST);
        then.status(500);
    });
    let deletes = server.mock(|when, then| {
        when.method(DELETE);
        then.status(500);
    });
    let cases = [
        vec!["card", "child", "add", "0", "--child", "456"],
        vec!["card", "child", "add", "123", "--child", "0"],
        vec!["card", "child", "add", "not-an-id", "--child", "456"],
        vec!["card", "child", "add", "123", "--child", "123"],
        vec!["card", "child", "remove", "123", "--child", "123"],
    ];

    for args in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(args)
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(
            stderr(&output).contains("card IDs must")
                || stderr(&output).contains("card ID must be a positive integer"),
            "unexpected error: {}",
            stderr(&output)
        );
    }

    assert_eq!(posts.calls(), 0);
    assert_eq!(deletes.calls(), 0);
}

#[test]
fn card_child_add_does_not_retry_failed_post() {
    let server = MockServer::start();
    let relation = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/children")
            .json_body_obj(&serde_json::json!({"card_id": 456}));
        then.status(429).body("rate limited");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "child", "add", "123", "--child", "456"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("429"));
    assert_eq!(relation.calls(), 1);
}

#[test]
fn card_child_add_help_documents_interface_and_retry_safety() {
    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .args(["card", "child", "add", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<PARENT_CARD_ID>"));
    assert!(stdout.contains("--child <CHILD_CARD_ID>"));
    assert!(stdout.contains("Failed POST requests are not retried automatically"));
}

#[test]
fn card_comment_add_posts_text_and_returns_created_comment_as_json() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/comments")
            .json_body_obj(&serde_json::json!({"text": "Ready for review"}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 456,
            "text": "Ready for review",
            "created": "2026-08-31T12:00:00.000Z",
            "updated": "2026-08-31T12:00:00.000Z",
            "card_id": 123,
            "author_id": 42
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "comment",
            "add",
            "123",
            "--text",
            "Ready for review",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["id"], 456);
    assert_eq!(json["text"], "Ready for review");
    assert_eq!(json["created"], "2026-08-31T12:00:00.000Z");
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_comment_add_confirms_created_comment_in_human_output() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/comments")
            .json_body_obj(&serde_json::json!({"text": "Shipped"}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 456,
            "text": "Shipped"
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "comment", "add", "123", "--text", "Shipped"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Added comment #456 to card #123.\n"
    );
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_comment_add_rejects_empty_text_before_request() {
    let server = MockServer::start();
    let post = server.mock(|when, then| {
        when.method(POST);
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 456,
            "text": "unexpected"
        }));
    });

    for text in ["", "   \t"] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(["card", "comment", "add", "123", "--text", text])
            .output()
            .unwrap();

        assert!(!output.status.success());
        assert!(stderr(&output).contains("comment text must not be empty"));
    }
    assert_eq!(post.calls(), 0);
}

#[test]
fn card_comment_add_rejects_invalid_card_id_before_request() {
    let server = MockServer::start();
    let post = server.mock(|when, then| {
        when.method(POST);
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 456,
            "text": "unexpected"
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "comment", "add", "0", "--text", "Hello"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("card ID must be a positive integer"));
    assert_eq!(post.calls(), 0);
}

#[test]
fn card_comment_add_does_not_retry_failed_post() {
    let server = MockServer::start();
    let post = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/comments")
            .json_body_obj(&serde_json::json!({"text": "Possibly created"}));
        then.status(429).body("rate limited");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "comment",
            "add",
            "123",
            "--text",
            "Possibly created",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("Kaiten API returned 429: rate limited"));
    assert_eq!(post.calls(), 1);
}

#[test]
fn card_comment_add_does_not_follow_post_redirect() {
    let server = MockServer::start();
    let redirected_post = server.mock(|when, then| {
        when.method(POST)
            .path("/duplicate")
            .json_body_obj(&serde_json::json!({"text": "Possibly duplicated"}));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 457,
            "text": "Possibly duplicated"
        }));
    });
    let redirect_location = server.url("/duplicate");
    let initial_post = server.mock(|when, then| {
        when.method(POST)
            .path("/cards/123/comments")
            .json_body_obj(&serde_json::json!({"text": "Possibly duplicated"}));
        then.status(307)
            .header("Location", redirect_location.as_str())
            .body("redirected");
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "comment",
            "add",
            "123",
            "--text",
            "Possibly duplicated",
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("Kaiten API returned 307: redirected"));
    assert_eq!(initial_post.calls(), 1);
    assert_eq!(redirected_post.calls(), 0);
}

#[test]
fn card_comment_add_help_documents_interface_and_retry_safety() {
    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .args(["card", "comment", "add", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("kten card comment add [OPTIONS] --text <TEXT> <CARD_ID>"));
    assert!(stdout.contains("Add one comment to an existing card"));
    assert!(stdout.contains("Failed POST requests are not retried automatically"));
    assert!(stdout.contains("--json"));
}

#[test]
fn card_create_posts_required_fields_and_prints_created_card() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/cards")
            .json_body_obj(&serde_json::json!({
                "title": "Fix login",
                "board_id": 2
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "description": null,
            "board_id": 2
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "create",
            "--title",
            "Fix login",
            "--board",
            "2",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"id\": 123"));
    assert!(stdout.contains("\"title\": \"Fix login\""));
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_create_maps_optional_fields_and_position() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/cards")
            .json_body_obj(&serde_json::json!({
                "title": "Release",
                "board_id": 2,
                "description": "Ship it",
                "column_id": 3,
                "lane_id": 4,
                "owner_id": 5,
                "responsible_id": 6,
                "due_date": "2026-07-31T15:00:00Z",
                "due_date_time_present": true,
                "asap": true,
                "position": 2
            }));
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 124,
            "title": "Release",
            "description": "Ship it",
            "board_id": 2,
            "column_id": 3,
            "lane_id": 4,
            "owner_id": 5,
            "responsible_id": 6
        }));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "card",
            "create",
            "--title",
            "Release",
            "--board",
            "2",
            "--description",
            "Ship it",
            "--column",
            "3",
            "--lane",
            "4",
            "--owner",
            "5",
            "--responsible",
            "6",
            "--due-date",
            "2026-07-31T15:00:00Z",
            "--due-date-time-present",
            "--asap",
            "--position",
            "last",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(mock.calls(), 1);
}

#[test]
fn search_includes_filters_and_description_request() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/cards")
            .query_param("query", "release")
            .query_param("space_id", "1")
            .query_param("board_id", "2")
            .query_param("limit", "3")
            .query_param("additional_card_fields", "description");
        then.status(200).json_body_obj(&serde_json::json!([
            {"id": 7, "title": "Release", "description": "Snippet"}
        ]));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args([
            "search", "release", "--space", "1", "--board", "2", "--limit", "3", "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(mock.calls(), 1);
}

#[test]
fn card_mine_fetches_all_current_user_roles_and_prints_lane() {
    let server = MockServer::start();
    let user = server.mock(|when, then| {
        when.method(GET).path("/users/current");
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 42,
            "full_name": "Alex Example"
        }));
    });
    let owner_cards = server.mock(|when, then| {
        when.method(GET)
            .path("/cards")
            .query_param("owner_ids", "42")
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
                "state": 2,
                "owner_id": 42,
                "lane_id": 30,
                "lane": {"id": 30, "title": "Backend"}
            }
        ]));
    });
    let responsible_cards = server.mock(|when, then| {
        when.method(GET)
            .path("/cards")
            .query_param("responsible_ids", "42")
            .query_param("states", "1,2")
            .query_param("archived", "false")
            .query_param("condition", "1")
            .query_param("limit", "20")
            .query_param("offset", "0");
        then.status(200).json_body_obj(&serde_json::json!([
            {
                "id": 124,
                "title": "Responsible card",
                "responsible_id": 42,
                "lane_id": 31,
                "lane": {"id": 31, "title": "Platform"}
            }
        ]));
    });
    let member_cards = server.mock(|when, then| {
        when.method(GET)
            .path("/cards")
            .query_param("member_ids", "42")
            .query_param("states", "1,2")
            .query_param("archived", "false")
            .query_param("condition", "1")
            .query_param("limit", "20")
            .query_param("offset", "0");
        then.status(200).json_body_obj(&serde_json::json!([
            {
                "id": 123,
                "title": "Implement card mine",
                "lane_id": 30,
                "lane": {"id": 30, "title": "Backend"}
            },
            {
                "id": 125,
                "title": "Member card",
                "lane_id": 32,
                "lane": {"id": 32, "title": "Support"}
            }
        ]));
    });

    let json_output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "mine", "--json"])
        .output()
        .unwrap();

    assert!(json_output.status.success(), "{}", stderr(&json_output));
    let json: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 3);
    assert_eq!(json[0]["id"], 123);
    assert_eq!(json[1]["id"], 124);
    assert_eq!(json[2]["id"], 125);
    assert_eq!(json[0]["lane_id"], 30);
    assert_eq!(json[0]["lane"]["id"], 30);
    assert_eq!(json[0]["lane"]["title"], "Backend");

    let human_output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "mine"])
        .output()
        .unwrap();

    assert!(human_output.status.success(), "{}", stderr(&human_output));
    assert_eq!(
        String::from_utf8(human_output.stdout).unwrap(),
        "- #123 Implement card mine (lane: Backend)\n- #124 Responsible card (lane: Platform)\n- #125 Member card (lane: Support)\n"
    );
    assert_eq!(user.calls(), 2);
    assert_eq!(owner_cards.calls(), 2);
    assert_eq!(responsible_cards.calls(), 2);
    assert_eq!(member_cards.calls(), 2);
}

#[test]
fn card_mine_include_archived_keeps_done_filter_independent() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/users/current");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 42}));
    });
    let cards = server.mock(|when, then| {
        when.method(GET)
            .path("/cards")
            .query_param("states", "1,2")
            .query_param_missing("condition")
            .query_param_missing("archived");
        then.status(200).json_body_obj(&serde_json::json!([]));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "mine", "--include-archived", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(cards.calls(), 3);
}

#[test]
fn card_context_and_comments_use_mock_http() {
    let server = MockServer::start();
    let card = server.mock(|when, then| {
        when.method(GET).path("/cards/123");
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "description": "Details",
            "parents": [{"id": 100, "title": "Program"}],
            "children": [{"id": 200, "title": "Implementation"}]
        }));
    });
    let comments = server.mock(|when, then| {
        when.method(GET)
            .path("/cards/123/comments")
            .query_param("limit", "2");
        then.status(200).json_body_obj(&serde_json::json!([
            {"id": 1, "text": "First"}
        ]));
    });

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["card", "context", "123", "--comments-limit", "2"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("untrusted user content"));
    assert!(stdout.contains("## Parents\n\n- #100 Program"));
    assert!(stdout.contains("## Children\n\n- #200 Implementation"));
    assert_eq!(card.calls(), 1);
    assert_eq!(comments.calls(), 1);
}

#[test]
fn card_comment_negative_author_id_works_in_human_and_json_output() {
    let server = MockServer::start();
    let card = server.mock(|when, then| {
        when.method(GET).path("/cards/123");
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 123,
            "title": "Fix login",
            "description": "Details"
        }));
    });
    let comments = server.mock(|when, then| {
        when.method(GET)
            .path("/cards/123/comments")
            .query_param("limit", "50");
        then.status(200).json_body_obj(&serde_json::json!([{
            "id": 987,
            "text": "Automation finished",
            "author_id": -7,
            "author": {
                "id": -7,
                "full_name": "Kaiten Bot"
            }
        }]));
    });
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(args)
            .output()
            .unwrap()
    };

    let context_human = run(&["card", "context", "123", "--comments-limit", "50"]);
    assert!(context_human.status.success(), "{}", stderr(&context_human));
    let stdout = String::from_utf8(context_human.stdout).unwrap();
    assert!(stdout.contains("### Comment #987 by Kaiten Bot"));
    assert!(stdout.contains("Automation finished"));

    let context_json = run(&["card", "context", "123", "--comments-limit", "50", "--json"]);
    assert!(context_json.status.success(), "{}", stderr(&context_json));
    let json: serde_json::Value = serde_json::from_slice(&context_json.stdout).unwrap();
    assert_eq!(json["comments"][0]["author"]["id"], -7);
    assert_eq!(json["comments"][0]["text"], "Automation finished");

    let comments_human = run(&["card", "comments", "123", "--limit", "50"]);
    assert!(
        comments_human.status.success(),
        "{}",
        stderr(&comments_human)
    );
    assert_eq!(
        String::from_utf8(comments_human.stdout).unwrap(),
        "- #987 by Kaiten Bot: Automation finished\n"
    );

    let comments_json = run(&["card", "comments", "123", "--limit", "50", "--json"]);
    assert!(comments_json.status.success(), "{}", stderr(&comments_json));
    let json: serde_json::Value = serde_json::from_slice(&comments_json.stdout).unwrap();
    assert_eq!(json[0]["author"]["id"], -7);
    assert_eq!(json[0]["text"], "Automation finished");

    assert_eq!(card.calls(), 2);
    assert_eq!(comments.calls(), 4);
}

#[test]
fn spaces_and_boards_use_mock_http() {
    let server = MockServer::start();
    let spaces = server.mock(|when, then| {
        when.method(GET).path("/spaces").query_param("limit", "20");
        then.status(200)
            .json_body_obj(&serde_json::json!([{"id": 1, "title": "Main"}]));
    });
    let space = server.mock(|when, then| {
        when.method(GET).path("/spaces/1");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 1, "title": "Main"}));
    });
    let boards = server.mock(|when, then| {
        when.method(GET).path("/spaces/1/boards");
        then.status(200)
            .json_body_obj(&serde_json::json!([{"id": 2, "title": "Dev"}]));
    });
    let board = server.mock(|when, then| {
        when.method(GET).path("/boards/2");
        then.status(200)
            .json_body_obj(&serde_json::json!({"id": 2, "title": "Dev"}));
    });

    for args in [
        vec!["space", "list"],
        vec!["space", "view", "1"],
        vec!["board", "list", "--space", "1"],
        vec!["board", "view", "2"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", stderr(&output));
    }

    assert_eq!(spaces.calls(), 1);
    assert_eq!(space.calls(), 1);
    assert_eq!(boards.calls(), 1);
    assert_eq!(board.calls(), 1);
}

#[test]
fn board_metadata_commands_use_mock_http() {
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

    for args in [
        vec!["board", "columns", "55843"],
        vec!["board", "lanes", "55843"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_kten"))
            .env("KTEN_HOSTNAME", "company.kaiten.ru")
            .env("KTEN_TOKEN", "secret-token")
            .env("KTEN_TEST_API_BASE", server.url(""))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", stderr(&output));
    }

    assert_eq!(columns.calls(), 1);
    assert_eq!(lanes.calls(), 1);
}

#[test]
fn board_structure_json_fetches_board_columns_and_lanes() {
    let server = MockServer::start();
    let board = server.mock(|when, then| {
        when.method(GET).path("/boards/55843");
        then.status(200).json_body_obj(&serde_json::json!({
            "id": 55843,
            "title": "Development"
        }));
    });
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

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_HOSTNAME", "company.kaiten.ru")
        .env("KTEN_TOKEN", "secret-token")
        .env("KTEN_TEST_API_BASE", server.url(""))
        .args(["board", "structure", "55843", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("\"title\": \"Development\""));
    assert!(stdout.contains("\"columns\""));
    assert!(stdout.contains("\"lanes\""));
    assert_eq!(board.calls(), 1);
    assert_eq!(columns.calls(), 1);
    assert_eq!(lanes.calls(), 1);
}

#[test]
fn skills_install_writes_default_skill_to_custom_path_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let skills_dir = dir.path().join("skills");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let installed = skills_dir.join("kten");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("✓ Installed {}", installed.display())
    );
    let skill_md = fs::read_to_string(installed.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("name: kten"));
    assert!(skill_md.contains("Kaiten CLI workflow"));
}

#[test]
fn skills_list_shows_bundled_skills_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "list"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Name\tSource\tDescription"));
    assert!(stdout.contains("kten\tbundled\tKaiten CLI workflow"));
    assert!(stdout.contains("kten-mcp\tbundled\tConfigure and use the kten stdio MCP server"));
}

#[test]
fn skills_install_writes_named_skill_to_custom_path_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let skills_dir = dir.path().join("skills");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "kten-mcp", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let installed = skills_dir.join("kten-mcp");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("✓ Installed {}", installed.display())
    );
    let skill_md = fs::read_to_string(installed.join("SKILL.md")).unwrap();
    assert!(skill_md.contains("name: kten-mcp"));
    assert!(skill_md.contains("kten stdio MCP server"));
}

#[test]
fn skills_install_existing_file_warns_and_exits_successfully_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let skills_dir = dir.path().join("skills");

    let first = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();
    assert!(first.status.success(), "{}", stderr(&first));

    let skill_md = skills_dir.join("kten").join("SKILL.md");
    fs::write(&skill_md, "local edit").unwrap();

    let second = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();

    assert!(second.status.success(), "{}", stderr(&second));
    assert_eq!(
        String::from_utf8(second.stdout).unwrap().trim(),
        format!(
            "! {} already exists. Use --force to overwrite.",
            skill_md.display()
        )
    );
    assert_eq!(fs::read_to_string(&skill_md).unwrap(), "local edit");
}

#[test]
fn skills_install_force_overwrites_existing_file_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let skills_dir = dir.path().join("skills");

    let first = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();
    assert!(first.status.success(), "{}", stderr(&first));

    let skill_md = skills_dir.join("kten").join("SKILL.md");
    fs::write(&skill_md, "local edit").unwrap();

    let forced = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "--force", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();

    assert!(forced.status.success(), "{}", stderr(&forced));
    let installed = skills_dir.join("kten");
    assert_eq!(
        String::from_utf8(forced.stdout).unwrap().trim(),
        format!("✓ Overwrote {}", installed.display())
    );
    assert!(
        fs::read_to_string(&skill_md)
            .unwrap()
            .contains("name: kten")
    );
}

#[test]
fn skills_install_defaults_to_current_git_project_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let project = dir.path().join("project");
    let nested = project.join("nested");
    fs::create_dir_all(project.join(".git")).unwrap();
    fs::create_dir_all(&nested).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .current_dir(&nested)
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let installed = project
        .canonicalize()
        .unwrap()
        .join(".agents")
        .join("skills")
        .join("kten");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("✓ Installed {}", installed.display())
    );
    assert!(installed.join("SKILL.md").exists());
}

#[test]
fn skills_install_global_uses_home_agents_skills_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let home = dir.path().join("home");
    fs::create_dir_all(&home).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .env("HOME", &home)
        .args(["skills", "install", "--global"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let installed = home.join(".agents").join("skills").join("kten");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        format!("✓ Installed {}", installed.display())
    );
    assert!(installed.join("SKILL.md").exists());
}

#[test]
fn skills_install_unknown_skill_fails_like_glab() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("config.toml");
    let skills_dir = dir.path().join("skills");

    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .env("KTEN_CONFIG", &config_path)
        .args(["skills", "install", "nope", "--path"])
        .arg(&skills_dir)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains(
        "Skill not found: unknown skill \"nope\". Run 'kten skills list' to see available skills."
    ));
}

#[test]
fn skills_install_help_matches_glab_style_interface() {
    let output = Command::new(env!("CARGO_BIN_EXE_kten"))
        .args(["skills", "install", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Install bundled `SKILL.md` files to `.agents/skills/`"));
    assert!(stdout.contains("By default, only the core `kten` skill is installed."));
    assert!(stdout.contains("kten skills install kten-mcp"));
    assert!(stdout.contains("-f, --force"));
    assert!(stdout.contains("-g, --global"));
    assert!(stdout.contains("--path"));
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}
