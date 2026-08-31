mod skills;

use std::{env, fs, io::Read, path::PathBuf, process::Command, str::FromStr};

use anyhow::Context;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use dialoguer::{Input, Password};
use kten_core::{
    CliConfigOverrides, Config, ConfigPaths, EditableConfigKey, EffectiveConfig, Error,
    KaitenClient, KaitenClientConfig, LimitKind, Limits, OutputFormat,
    models::{CreateCardRequest, MineCardsFilters, SearchFilters, UpdateCardRequest},
    render,
};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};

#[derive(Debug, Parser)]
#[command(name = "kten", version, about = "Unofficial Kaiten developer tool")]
struct Cli {
    #[arg(long, global = true, env = "KTEN_HOSTNAME")]
    hostname: Option<String>,
    #[arg(long, global = true, env = "KTEN_TOKEN", hide_env_values = true)]
    token: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
enum Commands {
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Card {
        #[command(subcommand)]
        command: CardCommand,
    },
    Search {
        query: String,
        #[arg(long)]
        space: Option<u64>,
        #[arg(long)]
        board: Option<u64>,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    Space {
        #[command(subcommand)]
        command: SpaceCommand,
    },
    Board {
        #[command(subcommand)]
        command: BoardCommand,
    },
    Completion {
        shell: CompletionShell,
    },
    #[command(about = "Manage kten agent skills")]
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
    Mcp,
}

#[derive(Debug, Clone, Subcommand)]
enum AuthCommand {
    Login {
        #[arg(long)]
        stdin: bool,
    },
    Status {
        #[arg(long)]
        hostname: Option<String>,
        #[arg(long, conflicts_with = "hostname")]
        all: bool,
        #[arg(long)]
        show_token: bool,
        #[arg(long)]
        json: bool,
    },
    Logout {
        #[arg(long)]
        hostname: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum ConfigCommand {
    Get { key: String },
    Set { key: String, value: String },
    Edit,
}

#[derive(Debug, Clone, Subcommand)]
enum CardCommand {
    Create {
        #[arg(long, help = "Card title (1-1024 characters)")]
        title: String,
        #[arg(long, help = "Destination board ID")]
        board: u64,
        #[arg(long, help = "Card description")]
        description: Option<String>,
        #[arg(long, help = "Destination column ID")]
        column: Option<u64>,
        #[arg(long, help = "Destination lane ID")]
        lane: Option<u64>,
        #[arg(long, help = "Owner user ID")]
        owner: Option<u64>,
        #[arg(long, help = "Responsible user ID")]
        responsible: Option<u64>,
        #[arg(long, help = "Deadline in ISO 8601 format")]
        due_date: Option<String>,
        #[arg(
            long,
            requires = "due_date",
            help = "Keep hours and minutes in the deadline"
        )]
        due_date_time_present: bool,
        #[arg(long, help = "Mark the card as ASAP")]
        asap: bool,
        #[arg(long, help = "Place the card first or last in its cell")]
        position: Option<CreatePosition>,
        #[arg(long)]
        json: bool,
    },
    Member {
        #[command(subcommand)]
        command: CardMemberCommand,
    },
    Update {
        id: u64,
        #[arg(
            long,
            required_unless_present = "priority",
            help = "Card description; pass an empty string to clear it"
        )]
        description: Option<String>,
        #[arg(long, required_unless_present = "description", help = "Card priority")]
        priority: Option<CardPriority>,
        #[arg(long)]
        json: bool,
    },
    View {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    Context {
        id: u64,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        comments_limit: Option<u32>,
    },
    Comments {
        id: u64,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    Mine {
        #[arg(long)]
        include_done: bool,
        #[arg(long)]
        include_archived: bool,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum CardMemberCommand {
    Add {
        id: u64,
        #[arg(long, help = "User ID to add as a card member")]
        user: u64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum SpaceCommand {
    List {
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    View {
        id: u64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum BoardCommand {
    List {
        #[arg(long)]
        space: u64,
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long)]
        json: bool,
    },
    View {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    Columns {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    Lanes {
        id: u64,
        #[arg(long)]
        json: bool,
    },
    Structure {
        id: u64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Subcommand)]
enum SkillsCommand {
    #[command(about = "List the available bundled agent skills. (EXPERIMENTAL)")]
    List,
    #[command(
        about = "Install kten's bundled agent skills. (EXPERIMENTAL)",
        long_about = "Install bundled `SKILL.md` files to `.agents/skills/`, the cross-agent standard defined by the Agent Skills specification. This works with GitLab Duo Agent Platform, Claude Code, Codex, Gemini CLI, and any other compliant agent.\n\nBy default, only the core `kten` skill is installed. Pass a positional `name` argument to install a specific bundled skill instead. Run `kten skills list` to see what is available.\n\nInstall scope:\n\n- By default, skills are installed for the current project, in `.agents/skills/` at the root of the current Git repository.\n- Use `--global` to install skills for the current user, in `~/.agents/skills/`.\n- Use `--path` to install skills to a custom directory. The path is resolved relative to the current working directory, not the repository root.\n\nTo overwrite an existing skill file, use `--force`.\n\nThis feature is an experiment and is not ready for production use. It might be unstable or removed at any time.",
        after_help = "Examples:\n  # Install the core kten skill in the current project (default)\n  kten skills install\n\n  # Install a specific bundled skill by name\n  kten skills install kten-mcp\n\n  # Install the core skill globally (user scope)\n  kten skills install --global\n\n  # Install a skill to a custom directory\n  kten skills install kten-mcp --path /path/to/skills\n\n  # Overwrite an existing skill file\n  kten skills install --force"
    )]
    Install {
        name: Option<String>,
        #[arg(short, long, help = "Overwrite existing skill files.")]
        force: bool,
        #[arg(
            short,
            long,
            conflicts_with = "path",
            help = "Install skills at user scope (~/.agents/skills/)."
        )]
        global: bool,
        #[arg(long, help = "Install skills to the directory at <path>.")]
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CreatePosition {
    First,
    Last,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CardPriority {
    Normal,
    High,
}

impl CardPriority {
    fn is_asap(self) -> bool {
        matches!(self, Self::High)
    }
}

impl CreatePosition {
    fn api_value(self) -> u8 {
        match self {
            Self::First => 1,
            Self::Last => 2,
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct AuthStatusItem {
    hostname: String,
    api_base: String,
    ca_bundle: Option<String>,
    selected: bool,
    token: String,
    valid: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("off")),
        )
        .with_writer(std::io::stderr)
        .with_span_events(FmtSpan::NONE)
        .init();
    run(Cli::parse()).await
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    if let Commands::Skills { command } = cli.command.clone() {
        return run_skills(command);
    }

    let paths = ConfigPaths::default();
    let mut config = Config::load(paths).context("failed to load config")?;
    match cli.command.clone() {
        Commands::Auth { command } => run_auth(command, &mut config, &cli).await,
        Commands::Config { command } => run_config(command, &mut config, &cli),
        Commands::Card { command } => {
            let effective = effective(&config, &cli)?;
            let client = client(&effective)?;
            run_card(command, &effective, &client).await
        }
        Commands::Search {
            query,
            space,
            board,
            limit,
            json,
        } => {
            let effective = effective(&config, &cli)?;
            let client = client(&effective)?;
            let cards = client
                .search_cards(SearchFilters {
                    query,
                    space_id: space,
                    board_id: board,
                    limit: Limits::validate(limit, LimitKind::Search)?,
                })
                .await?;
            print_data(json, || render::cards_human(&cards), &cards)
        }
        Commands::Space { command } => {
            let effective = effective(&config, &cli)?;
            let client = client(&effective)?;
            run_space(command, &client).await
        }
        Commands::Board { command } => {
            let effective = effective(&config, &cli)?;
            let client = client(&effective)?;
            run_board(command, &client).await
        }
        Commands::Completion { shell } => {
            let mut command = Cli::command();
            let shell = match shell {
                CompletionShell::Bash => Shell::Bash,
                CompletionShell::Zsh => Shell::Zsh,
                CompletionShell::Fish => Shell::Fish,
                CompletionShell::Powershell => Shell::PowerShell,
                CompletionShell::Elvish => Shell::Elvish,
            };
            clap_complete::generate(shell, &mut command, "kten", &mut std::io::stdout());
            Ok(())
        }
        Commands::Skills { command } => run_skills(command),
        Commands::Mcp => {
            let effective = effective(&config, &cli)?;
            kten_mcp::serve_stdio(KaitenClientConfig::try_from(&effective)?).await
        }
    }
}

fn run_skills(command: SkillsCommand) -> anyhow::Result<()> {
    match command {
        SkillsCommand::List => {
            println!("Name\tSource\tDescription");
            for skill in skills::list_skills() {
                println!("{}\t{}\t{}", skill.name, skill.source, skill.description);
            }
            Ok(())
        }
        SkillsCommand::Install {
            name,
            force,
            global,
            path,
        } => {
            let skill_name = name.as_deref().unwrap_or(skills::DEFAULT_SKILL);
            let skill = skills::find_skill(skill_name)?;
            let scope = if let Some(path) = path {
                skills::InstallScope::Custom(path)
            } else if global {
                skills::InstallScope::Global
            } else {
                skills::InstallScope::Project
            };
            match skills::install_skill(skill, scope, force)? {
                skills::InstallOutcome::Installed(path) => {
                    println!("✓ Installed {}", path.display());
                }
                skills::InstallOutcome::Overwrote(path) => {
                    println!("✓ Overwrote {}", path.display());
                }
                skills::InstallOutcome::AlreadyExists(path) => {
                    println!(
                        "! {} already exists. Use --force to overwrite.",
                        path.display()
                    );
                }
            }
            Ok(())
        }
    }
}

async fn run_auth(command: AuthCommand, config: &mut Config, cli: &Cli) -> anyhow::Result<()> {
    match command {
        AuthCommand::Login { stdin } => {
            let hostname = cli.hostname.clone().unwrap_or_else(|| {
                Input::new()
                    .with_prompt("Kaiten hostname")
                    .interact_text()
                    .unwrap_or_default()
            });
            let token = if let Some(value) = cli.token.clone() {
                if stdin {
                    anyhow::bail!("--token and --stdin are mutually exclusive");
                }
                value
            } else if stdin {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                if input.ends_with('\n') {
                    input.pop();
                    if input.ends_with('\r') {
                        input.pop();
                    }
                }
                input
            } else {
                Password::new().with_prompt("Kaiten token").interact()?
            };
            let ca_bundle = env::var("KTEN_CA_BUNDLE")
                .ok()
                .or_else(|| config.file.ca_bundle.clone());
            let probe = EffectiveConfig {
                hostname: hostname.clone(),
                api_base: format!("https://{hostname}/api/latest"),
                token: Some(token.clone()),
                ca_bundle,
                output: OutputFormat::Human,
                comments_limit: 10,
            };
            client(&probe)?.validate_auth().await?;
            config.login(hostname, token)?;
            config.save()?;
            println!("Logged in.");
            Ok(())
        }
        AuthCommand::Status {
            hostname,
            all,
            show_token,
            json,
        } => {
            let mut statuses = Vec::new();
            if all {
                for state in config.auth_state_all(show_token) {
                    let resolved = config.effective(CliConfigOverrides {
                        hostname: Some(state.hostname.clone()),
                        token: cli.token.clone(),
                        ca_bundle: None,
                        output: None,
                        comments_limit: None,
                    });
                    let ca_bundle = resolved.as_ref().ok().and_then(|cfg| cfg.ca_bundle.clone());
                    let valid = if let Ok(cfg) = resolved {
                        if let Ok(c) = client(&cfg) {
                            c.validate_auth().await.is_ok()
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    statuses.push(AuthStatusItem {
                        hostname: state.hostname,
                        api_base: state.api_base,
                        ca_bundle,
                        selected: state.is_default,
                        token: state.token_preview.unwrap_or_default(),
                        valid,
                    });
                    if !valid {
                        anyhow::bail!("authentication validation failed");
                    }
                }
            } else {
                let selected = hostname
                    .or(cli.hostname.clone())
                    .or_else(|| config.file.default_hostname.clone())
                    .context("missing hostname")?;
                let state = config.auth_state_for(&selected, show_token)?;
                let resolved = config.effective(CliConfigOverrides {
                    hostname: Some(state.hostname.clone()),
                    token: cli.token.clone(),
                    ca_bundle: None,
                    output: None,
                    comments_limit: None,
                });
                let ca_bundle = resolved.as_ref().ok().and_then(|cfg| cfg.ca_bundle.clone());
                let valid = if let Ok(cfg) = resolved {
                    if let Ok(c) = client(&cfg) {
                        c.validate_auth().await.is_ok()
                    } else {
                        false
                    }
                } else {
                    false
                };
                statuses.push(AuthStatusItem {
                    hostname: state.hostname,
                    api_base: state.api_base,
                    ca_bundle,
                    selected: state.is_default,
                    token: state.token_preview.unwrap_or_default(),
                    valid,
                });
                if !valid {
                    anyhow::bail!("authentication validation failed");
                }
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
            } else {
                for item in statuses {
                    println!("Hostname: {}", item.hostname);
                    println!("API base: {}", item.api_base);
                    println!("CA bundle: {}", item.ca_bundle.as_deref().unwrap_or(""));
                    println!("Selected: {}", if item.selected { "*" } else { " " });
                    println!("Validation: {}", if item.valid { "ok" } else { "failed" });
                    println!(
                        "Token: {}",
                        if item.token.is_empty() {
                            "not configured"
                        } else {
                            item.token.as_str()
                        }
                    );
                    println!();
                }
            }
            Ok(())
        }
        AuthCommand::Logout { hostname } => {
            config.logout(hostname.or(cli.hostname.clone()))?;
            config.save()?;
            println!("Logged out.");
            Ok(())
        }
    }
}

fn run_config(command: ConfigCommand, config: &mut Config, cli: &Cli) -> anyhow::Result<()> {
    match command {
        ConfigCommand::Get { key } => {
            println!("{}", config_value(config, cli, &key)?);
            Ok(())
        }
        ConfigCommand::Set { key, value } => {
            let key = EditableConfigKey::from_str(&key)?;
            config.set(key, value)?;
            config.save()?;
            println!("Updated.");
            Ok(())
        }
        ConfigCommand::Edit => {
            edit_config(config)?;
            let reloaded =
                Config::load(config.paths.clone()).context("failed to reload config after edit")?;
            *config = reloaded;
            println!("Updated.");
            Ok(())
        }
    }
}

async fn run_card(
    command: CardCommand,
    effective: &EffectiveConfig,
    client: &KaitenClient,
) -> anyhow::Result<()> {
    match command {
        CardCommand::Create {
            title,
            board,
            description,
            column,
            lane,
            owner,
            responsible,
            due_date,
            due_date_time_present,
            asap,
            position,
            json,
        } => {
            let card = client
                .create_card(&CreateCardRequest {
                    title,
                    board_id: board,
                    description,
                    column_id: column,
                    lane_id: lane,
                    owner_id: owner,
                    responsible_id: responsible,
                    due_date,
                    due_date_time_present: due_date_time_present.then_some(true),
                    asap: asap.then_some(true),
                    position: position.map(CreatePosition::api_value),
                })
                .await?;
            print_data(
                json,
                || render::card_human(&card, &effective.card_url(card.id)),
                &card,
            )
        }
        CardCommand::Member { command } => match command {
            CardMemberCommand::Add { id, user, json } => {
                let member = client.add_card_member(id, user).await?;
                print_data(json, || render::card_member_human(id, &member), &member)
            }
        },
        CardCommand::View { id, json } => {
            let card = client.card(id).await?;
            print_data(
                json,
                || render::card_human(&card, &effective.card_url(id)),
                &card,
            )
        }
        CardCommand::Update {
            id,
            description,
            priority,
            json,
        } => {
            let card = client
                .update_card(
                    id,
                    &UpdateCardRequest {
                        description: description
                            .map(|description| (!description.is_empty()).then_some(description)),
                        asap: priority.map(CardPriority::is_asap),
                    },
                )
                .await?;
            print_data(
                json,
                || render::card_human(&card, &effective.card_url(id)),
                &card,
            )
        }
        CardCommand::Context {
            id,
            json,
            comments_limit,
        } => {
            let context = client
                .card_context(
                    id,
                    Limits::validate(
                        comments_limit.or(Some(effective.comments_limit)),
                        LimitKind::Comments,
                    )?,
                )
                .await?;
            print_data(json, || render::context_markdown(&context), &context)
        }
        CardCommand::Comments { id, limit, json } => {
            let comments = client
                .comments(id, Limits::validate(limit, LimitKind::Comments)?)
                .await?;
            print_data(json, || render::comments_human(&comments), &comments)
        }
        CardCommand::Mine {
            include_done,
            include_archived,
            limit,
            offset,
            json,
        } => {
            let cards = client
                .cards_mine(MineCardsFilters {
                    limit: Limits::validate(limit, LimitKind::Search)?,
                    include_done,
                    include_archived,
                    offset,
                })
                .await?;
            print_data(json, || render::mine_cards_human(&cards), &cards)
        }
    }
}

async fn run_space(command: SpaceCommand, client: &KaitenClient) -> anyhow::Result<()> {
    match command {
        SpaceCommand::List { limit, json } => {
            let spaces = client
                .spaces(Limits::validate(limit, LimitKind::List)?)
                .await?;
            print_data(json, || render::spaces_human(&spaces), &spaces)
        }
        SpaceCommand::View { id, json } => {
            let space = client.space(id).await?;
            print_data(json, || render::space_human(&space), &space)
        }
    }
}

async fn run_board(command: BoardCommand, client: &KaitenClient) -> anyhow::Result<()> {
    match command {
        BoardCommand::List { space, limit, json } => {
            let boards = client
                .boards(space, Limits::validate(limit, LimitKind::List)?)
                .await?;
            print_data(json, || render::boards_human(&boards), &boards)
        }
        BoardCommand::View { id, json } => {
            let board = client.board(id).await?;
            print_data(json, || render::board_human(&board), &board)
        }
        BoardCommand::Columns { id, json } => {
            let columns = client.columns(id).await?;
            print_data(json, || render::columns_human(&columns), &columns)
        }
        BoardCommand::Lanes { id, json } => {
            let lanes = client.lanes(id).await?;
            print_data(json, || render::lanes_human(&lanes), &lanes)
        }
        BoardCommand::Structure { id, json } => {
            let structure = client.board_structure(id).await?;
            print_data(
                json,
                || render::board_structure_human(&structure),
                &structure,
            )
        }
    }
}

fn effective(config: &Config, cli: &Cli) -> anyhow::Result<EffectiveConfig> {
    config
        .effective(CliConfigOverrides {
            hostname: cli.hostname.clone(),
            token: cli.token.clone(),
            ca_bundle: None,
            output: None,
            comments_limit: None,
        })
        .context("failed to resolve effective config")
}

fn client(config: &EffectiveConfig) -> anyhow::Result<KaitenClient> {
    Ok(KaitenClient::new(KaitenClientConfig::try_from(config)?)?)
}

fn print_data<T: serde::Serialize>(
    json: bool,
    human: impl FnOnce() -> String,
    value: &T,
) -> anyhow::Result<()> {
    if json {
        print!("{}", render::json(value)?);
    } else {
        print!("{}", human());
    }
    Ok(())
}

fn config_value(config: &Config, cli: &Cli, key: &str) -> anyhow::Result<String> {
    match key {
        "default_hostname" => Ok(cli
            .hostname
            .clone()
            .or_else(|| config.file.default_hostname.clone())
            .unwrap_or_default()),
        "token" => match config.effective(CliConfigOverrides {
            hostname: cli.hostname.clone(),
            token: cli.token.clone(),
            ca_bundle: None,
            output: None,
            comments_limit: None,
        }) {
            Ok(effective) => Ok(effective
                .token
                .map(|_| "<redacted>".to_string())
                .unwrap_or_default()),
            Err(Error::MissingHostname) => Ok(String::new()),
            Err(err) => Err(err.into()),
        },
        "ca_bundle" => Ok(env::var("KTEN_CA_BUNDLE")
            .ok()
            .or_else(|| config.file.ca_bundle.clone())
            .unwrap_or_default()),
        "output" => Ok(config
            .file
            .output
            .unwrap_or(OutputFormat::Human)
            .to_string()),
        "comments_limit" => Ok(config
            .file
            .comments_limit
            .unwrap_or_else(|| Limits::default_for(LimitKind::Comments))
            .to_string()),
        _ => anyhow::bail!("invalid config key: {key}"),
    }
}

fn edit_config(config: &Config) -> anyhow::Result<()> {
    if let Some(parent) = config.paths.config_file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;
    }
    if !config.paths.config_file.exists() {
        config.save().context("failed to initialize config file")?;
    }

    let editor = env::var("VISUAL")
        .ok()
        .or_else(|| env::var("EDITOR").ok())
        .ok_or_else(|| anyhow::anyhow!("set VISUAL or EDITOR to use `kten config edit`"))?;
    let mut parts = shell_words::split(&editor).context("failed to parse editor command")?;
    if parts.is_empty() {
        anyhow::bail!("set VISUAL or EDITOR to use `kten config edit`");
    }
    let program = parts.remove(0);
    let status = Command::new(program)
        .args(parts)
        .arg(&config.paths.config_file)
        .status()
        .context("failed to launch editor")?;
    if !status.success() {
        anyhow::bail!("editor exited with status: {status}");
    }
    Ok(())
}
