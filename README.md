# kten

`kten` is an unofficial Kaiten developer tool written in Rust.
It is not affiliated with, endorsed by, or supported by Kaiten.

The tool is CLI-first and also provides a stdio MCP server for AI agents.
It covers auth, config, card reads and creation, comments, search, spaces,
boards, shell completion, and MCP tools.

## Install

Prebuilt binaries for Apple Silicon Macs are available on
[GitHub Releases](https://github.com/sonr09/kaiten-kten/releases):

```sh
curl -fLO https://github.com/sonr09/kaiten-kten/releases/latest/download/kten-aarch64-apple-darwin.tar.gz
curl -fLO https://github.com/sonr09/kaiten-kten/releases/latest/download/kten-aarch64-apple-darwin.tar.gz.sha256
shasum -a 256 -c kten-aarch64-apple-darwin.tar.gz.sha256
tar -xzf kten-aarch64-apple-darwin.tar.gz
sudo install -m 0755 kten /usr/local/bin/kten
```

From source:

```sh
cargo install --path crates/kten-cli
```

The required Rust toolchain is pinned in `rust-toolchain.toml` to Rust 1.95.0.

## Auth

`kten` supports multiple host profiles. Use the full Kaiten host, for example
`company.kaiten.ru`.

```sh
kten auth login
kten auth login --hostname company.kaiten.ru --stdin < token.txt
kten auth status
kten auth status --all
kten auth status --hostname company.kaiten.ru --show-token
kten auth logout --hostname company.kaiten.ru
```

Environment variables override stored config:

```sh
KTEN_HOSTNAME=company.kaiten.ru
KTEN_TOKEN=...
KTEN_CA_BUNDLE=/path/to/corporate-ca.pem
KTEN_CONFIG_DIR=~/.config/kten
KTEN_CONFIG=~/.config/kten/config.toml
```

Config precedence is CLI auth flags, then environment, then the config file.
`KTEN_CONFIG` points to a config file and takes precedence over
`KTEN_CONFIG_DIR`, which points to a directory containing `config.toml`.

Use config commands to read and edit persisted settings:

```sh
kten config set default_hostname company.kaiten.ru
kten config set ca_bundle /path/to/corporate-ca.pem
kten config get default_hostname
kten config edit
```

## Corporate Certificates (Internal CA)

`kten` keeps TLS certificate verification enabled and does not support disabling
verification.

If your corporate Kaiten uses certificates signed by an internal CA, first try
installing that CA into your OS trust store. `kten` uses rustls with native OS
roots enabled.

If your environment cannot provide the corporate CA via OS trust store, set a
custom CA bundle (PEM):

```sh
kten config set ca_bundle /path/to/corporate-ca.pem
kten card view 12345

KTEN_CA_BUNDLE=/path/to/corporate-ca.pem kten card view 12345
```

## CLI Examples

Human-readable output is the default. Data commands support `--json`.

```sh
kten card view 12345
kten card view 12345 --json
kten card create --title "Fix login" --board 34
kten card update 12345 --description "Updated acceptance criteria"
kten card update 12345 --description "" # clear description
kten card member add 12345 --user 42
kten card create --title "Fix login" --board 34 --column 56 --lane 78 --responsible 90 --json
kten card context 12345 --comments-limit 20
kten card comments 12345 --limit 10 --json
kten card mine
kten card mine --include-done --limit 50 --json
kten search "release blocker" --space 12 --board 34 --limit 20
kten space list
kten space view 12 --json
kten board list --space 12
kten board view 34
kten board columns 55843
kten board lanes 55843
kten board structure 55843 --json
kten completion zsh
```

## Kaiten API Coverage

`kten` talks to the official Kaiten JSON REST API. The default API base for a
host profile is:

```text
https://company.kaiten.ru/api/latest
```

Requests use bearer-token authentication:

```text
Authorization: Bearer <token>
```

Kaiten's public API documentation is available at
<https://developers.kaiten.ru/>. The full Kaiten API includes read and write
operations for spaces, boards, columns, lanes, cards, comments, users, groups,
roles, tags, card types, custom properties, checklists, files, time logs,
service desks, automations, audit logs, webhooks, and other product areas.

`kten` intentionally exposes only a focused subset. Card creation, card
description updates, and adding card members are its supported write operations;
all other commands are
read-only:

| `kten` feature | Kaiten API request |
| --- | --- |
| Auth validation | `GET /users/current` |
| Card view | `GET /cards/{card_id}` |
| Card creation | `POST /cards` |
| Card description update | `PATCH /cards/{card_id}` |
| Add card member | `POST /cards/{card_id}/members` |
| Card search | `GET /cards` |
| My cards | `GET /users/current` and `GET /cards` |
| Card context | `GET /cards/{card_id}` and `GET /cards/{card_id}/comments` |
| Card comments | `GET /cards/{card_id}/comments` |
| Space list | `GET /spaces` |
| Space view | `GET /spaces/{space_id}` |
| Board list | `GET /spaces/{space_id}/boards` |
| Board view | `GET /boards/{board_id}` |
| Board columns | `GET /boards/{board_id}/columns` |
| Board lanes | `GET /boards/{board_id}/lanes` |
| Board structure | the board, columns, and lanes requests above |

Card search uses `GET /cards` with filters such as `query`, `space_id`,
`board_id`, `limit`, and `additional_card_fields=description`. Kaiten also
supports broader card filters such as responsible users, members, owners,
columns, lanes, states, archive status, tags, types, date ranges, and
pagination; `kten` adds only the filters that are currently exposed by the CLI
and MCP tools.

`kten card mine` resolves the authenticated Kaiten user and lists active,
non-archived cards where that user is the responsible person.

Use `kten board structure <board-id> --json` with card fields such as
`board_id`, `column_id`, and `lane_id` when you need to map cards to
human-readable board columns and lanes. `kten` exposes the primitives and leaves
workflow-specific joins to users or agents.

## MCP

Run the stdio MCP server with:

```sh
kten mcp
```

Available tools:

- `kten_create_card`
- `kten_get_card_context`
- `kten_search_cards`
- `kten_get_comments`
- `kten_list_spaces`
- `kten_get_space`
- `kten_list_boards`
- `kten_get_board`

Claude Desktop:

```json
{
  "mcpServers": {
    "kten": {
      "command": "kten",
      "args": ["mcp"],
      "env": {
        "KTEN_HOSTNAME": "company.kaiten.ru",
        "KTEN_TOKEN": "..."
      }
    }
  }
}
```

Cursor:

```json
{
  "mcpServers": {
    "kten": {
      "command": "kten",
      "args": ["mcp"],
      "env": {
        "KTEN_HOSTNAME": "company.kaiten.ru",
        "KTEN_TOKEN": "..."
      }
    }
  }
}
```

Codex:

```toml
[mcp_servers.kten]
command = "kten"
args = ["mcp"]
env = { KTEN_HOSTNAME = "company.kaiten.ru", KTEN_TOKEN = "..." }
```

opencode:

```json
{
  "mcp": {
    "kten": {
      "type": "local",
      "command": ["kten", "mcp"],
      "environment": {
        "KTEN_HOSTNAME": "company.kaiten.ru",
        "KTEN_TOKEN": "..."
      }
    }
  }
}
```

## Agent Skills

`kten` can install bundled Agent Skills so AI agents can discover how to use the
CLI and MCP server safely.

```sh
kten skills list
kten skills install
kten skills install kten-mcp
kten skills install --global
kten skills install --path /path/to/skills
kten skills install --force
```

By default, `kten skills install` writes the core `kten` skill to
`.agents/skills/` at the root of the current Git repository. Use `--global` for
`~/.agents/skills/` or `--path` for a custom skills directory.

## Development

Validation commands:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Card creation, card description updates, and adding card members are the supported Kaiten write operations. `kten` does not
automatically retry creation requests because a retry could create a duplicate.
It has no telemetry and no crates.io publishing configuration.

## License

Licensed under the [MIT License](LICENSE).
