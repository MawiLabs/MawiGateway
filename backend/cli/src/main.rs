//! # `mg` — MawiGateway CLI
//!
//! Operate the gateway from a terminal or CI script. Talks to the
//! gateway's REST API via the shared `mawi-client` crate, so when the
//! API surface changes you only update one file (`backend/client/src/
//! lib.rs`) and the CLI rebuilds against the new types.
//!
//! ## Auth precedence
//!
//! 1. `--api-key` flag (highest priority — useful in CI)
//! 2. `MG_API_KEY` environment variable
//! 3. `~/.mawigateway/config.yaml` (written by `mg auth login`)
//!
//! ## Output
//!
//! Default output is human-readable tables/colored text. Pass `--json`
//! globally to get JSON-on-stdout for shell pipelines.

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use mawi_client::{
    Client, CreateMcpServer, CreateModel, CreateProvider, CreateService, ChatMessage,
    ChatRequest, UpdateService,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;

mod config;

// ---------------------------------------------------------------------------
// CLI shape — clap derives the parser from these structs. Subcommands are
// grouped so output of `mg --help` reads like a noun-verb taxonomy.
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "mg",
    version,
    about = "Operate MawiGateway from the command line.",
    long_about = "mg — manage providers, models, services, MCP servers, and API keys.\n\
                  Auth: --api-key flag, MG_API_KEY env var, or ~/.mawigateway/config.yaml.\n\
                  Add --json to any command for machine-readable output."
)]
struct Cli {
    /// Override the gateway base URL. Defaults to MG_GATEWAY_URL or
    /// http://localhost:8030.
    #[arg(long, env = "MG_GATEWAY_URL", global = true)]
    gateway_url: Option<String>,

    /// API key for authentication. Falls back to MG_API_KEY env var
    /// then to ~/.mawigateway/config.yaml.
    #[arg(long, env = "MG_API_KEY", global = true)]
    api_key: Option<String>,

    /// Emit JSON instead of human tables.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Authenticate this CLI against a gateway.
    #[command(subcommand)]
    Auth(AuthCmd),

    /// Manage AI providers (OpenAI, Anthropic, Azure, …).
    #[command(subcommand)]
    Providers(ProvidersCmd),

    /// Manage models routed through providers.
    #[command(subcommand)]
    Models(ModelsCmd),

    /// Manage routing services (pools, agentic).
    #[command(subcommand)]
    Services(ServicesCmd),

    /// Manage API keys for programmatic access.
    #[command(subcommand)]
    Keys(KeysCmd),

    /// Manage MCP servers consumed by the gateway.
    #[command(subcommand)]
    Mcp(McpCmd),

    /// Inspect recent request logs.
    Logs {
        /// Number of log entries to fetch.
        #[arg(long, default_value_t = 50)]
        limit: u32,
    },

    /// Show usage analytics.
    Analytics {
        /// Time range: 24h | 7d | 30d.
        #[arg(long, default_value = "24h")]
        range: String,
    },

    /// Read the append-only audit log (#80). Filters compose with
    /// AND; pass any subset.
    Audit {
        /// Filter to one action (e.g. service.delete, api_key.create).
        #[arg(long)]
        action: Option<String>,
        /// Filter to one resource (e.g. service:text-default).
        #[arg(long)]
        resource: Option<String>,
        /// Filter to one user id.
        #[arg(long)]
        user: Option<String>,
        /// RFC 3339 timestamp (lower bound, inclusive).
        #[arg(long)]
        since: Option<String>,
        /// RFC 3339 timestamp (upper bound, inclusive).
        #[arg(long)]
        until: Option<String>,
        /// Page size (1..=200). Default 50.
        #[arg(long, default_value_t = 50)]
        limit: u32,
        /// Pagination offset.
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },

    /// One-shot chat completion against a service.
    Chat {
        /// Service name to route through.
        #[arg(long, default_value = "text-default")]
        service: String,
        /// Prompt text.
        prompt: String,
        /// Optional max output tokens.
        #[arg(long)]
        max_tokens: Option<u32>,
    },

    /// Apply or validate a YAML config (mawigateway.yaml schema).
    #[command(subcommand)]
    Config(ConfigCmd),

    /// Print the current authenticated user.
    Whoami,

    /// Print the resolved gateway URL and auth source (debugging).
    Doctor,
}

#[derive(Subcommand, Debug)]
enum AuthCmd {
    /// Save an API key to ~/.mawigateway/config.yaml.
    Login {
        /// API key to store. If omitted, prompts via stdin.
        #[arg(long)]
        api_key: Option<String>,
        /// Gateway URL to associate with the key.
        #[arg(long, default_value = "http://localhost:8030")]
        gateway_url: String,
    },
    /// Forget the saved API key.
    Logout,
    /// Print the path of the config file.
    Where,
}

#[derive(Subcommand, Debug)]
enum ProvidersCmd {
    List,
    Add {
        /// Friendly name (e.g. "OpenAI Production").
        #[arg(long)]
        name: String,
        /// Provider type: openai|anthropic|azure|google|xai|mistral|elevenlabs|selfhosted|...
        #[arg(long, name = "type")]
        provider_type: String,
        /// API key. Required for hosted providers.
        #[arg(long)]
        api_key: Option<String>,
        /// Endpoint override (Azure / self-hosted).
        #[arg(long)]
        endpoint: Option<String>,
        /// API version (Azure).
        #[arg(long)]
        api_version: Option<String>,
    },
    Remove { id: Uuid },
}

#[derive(Subcommand, Debug)]
enum ModelsCmd {
    List,
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        provider: Uuid,
        #[arg(long, default_value = "text")]
        modality: String,
        #[arg(long)]
        endpoint: Option<String>,
        #[arg(long)]
        api_version: Option<String>,
        #[arg(long)]
        api_key: Option<String>,
    },
    Remove { id: Uuid },
}

#[derive(Subcommand, Debug)]
enum ServicesCmd {
    List,
    Create {
        #[arg(long)]
        name: String,
        /// Service type: POOL | AGENTIC.
        #[arg(long, name = "type", default_value = "POOL")]
        service_type: String,
        /// Routing strategy: weighted_random | least_cost | least_latency | health | none | planner.
        #[arg(long, default_value = "weighted_random")]
        strategy: String,
        /// Modality (text | image | audio | …).
        #[arg(long, default_value = "text")]
        modality: String,
        /// Comma-separated model IDs to attach.
        #[arg(long, value_delimiter = ',')]
        models: Vec<Uuid>,
        #[arg(long)]
        description: Option<String>,
        /// Comma-separated alternate names that route to this service
        /// (e.g. `--aliases gpt-4o,claude-3-5-sonnet`). Lets OpenAI /
        /// Anthropic SDK users target the service via familiar model
        /// names while still going through MawiGateway's routing.
        #[arg(long, value_delimiter = ',')]
        aliases: Vec<String>,
    },
    /// Update an existing service. Every flag is optional — only the
    /// ones you pass get applied. Pass `--clear-aliases` to wipe the
    /// alias list, or `--aliases gpt-4o,...` to replace it.
    Update {
        name: String,
        #[arg(long, name = "type")]
        service_type: Option<String>,
        #[arg(long)]
        strategy: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        planner_model_id: Option<String>,
        #[arg(long)]
        system_prompt: Option<String>,
        #[arg(long)]
        max_iterations: Option<u32>,
        /// Replace the alias list with these (comma-separated).
        #[arg(long, value_delimiter = ',', conflicts_with = "clear_aliases")]
        aliases: Option<Vec<String>>,
        /// Clear all aliases. Equivalent to `--aliases ''`.
        #[arg(long)]
        clear_aliases: bool,
    },
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
enum KeysCmd {
    List,
    Create {
        #[arg(long)]
        name: String,
        /// Comma-separated scope list (#78). Defaults to `admin` when
        /// omitted (full access). Examples:
        ///   --scopes read                       # read-only key
        ///   --scopes chat                       # any chat call
        ///   --scopes chat:gpt-4o                # only that service
        ///   --scopes config:write,read          # CI deploy key
        #[arg(long, value_delimiter = ',')]
        scopes: Vec<String>,
    },
    Revoke { id: String },
}

#[derive(Subcommand, Debug)]
enum McpCmd {
    List,
    Add {
        #[arg(long)]
        name: String,
        /// Server type: docker | stdio | sse.
        #[arg(long, name = "type")]
        server_type: McpType,
        /// Docker image, command path, or SSE URL.
        #[arg(long)]
        image_or_command: String,
    },
    Connect { id: Uuid },
    Remove { id: Uuid },
}

#[derive(ValueEnum, Clone, Debug)]
enum McpType {
    Docker,
    Stdio,
    Sse,
}

impl McpType {
    fn as_str(&self) -> &'static str {
        match self {
            McpType::Docker => "docker",
            McpType::Stdio => "stdio",
            McpType::Sse => "sse",
        }
    }
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// POST a YAML file's contents to /v1/config/apply.
    Apply { file: PathBuf },
    /// Lint a YAML file locally without contacting the server.
    Validate { file: PathBuf },
}

// ---------------------------------------------------------------------------
// Auth resolution + client construction.
// ---------------------------------------------------------------------------

fn resolve_client(cli: &Cli) -> Result<Client> {
    let stored = config::load().ok();
    let url = cli
        .gateway_url
        .clone()
        .or_else(|| stored.as_ref().map(|c| c.gateway_url.clone()))
        .unwrap_or_else(|| "http://localhost:8030".to_string());
    let key = cli
        .api_key
        .clone()
        .or_else(|| stored.as_ref().and_then(|c| c.api_key.clone()));
    Ok(Client::new(url, key))
}

// ---------------------------------------------------------------------------
// Output helpers. Two modes: JSON (for scripting) and tabled (for humans).
// ---------------------------------------------------------------------------

fn out_json(json: bool, value: &impl Serialize) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).context("serialize json output")?
        );
    }
    Ok(())
}

fn print_table(headers: &[&str], rows: Vec<Vec<String>>) {
    let mut t = Table::new();
    t.load_preset(UTF8_FULL);
    t.set_header(headers.iter().map(|h| Cell::new(h)));
    for row in rows {
        t.add_row(row.into_iter().map(Cell::new));
    }
    println!("{}", t);
}

fn ok(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg);
}

fn err(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg);
}

// ---------------------------------------------------------------------------
// Entrypoint. Each subcommand dispatches to a focused handler so the main
// match stays scannable.
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Auth(a) => handle_auth(a).await,
        Command::Whoami => handle_whoami(&cli).await,
        Command::Doctor => handle_doctor(&cli).await,
        Command::Providers(p) => handle_providers(&cli, p).await,
        Command::Models(m) => handle_models(&cli, m).await,
        Command::Services(s) => handle_services(&cli, s).await,
        Command::Keys(k) => handle_keys(&cli, k).await,
        Command::Mcp(m) => handle_mcp(&cli, m).await,
        Command::Logs { limit } => handle_logs(&cli, *limit).await,
        Command::Analytics { range } => handle_analytics(&cli, range).await,
        Command::Audit {
            action,
            resource,
            user,
            since,
            until,
            limit,
            offset,
        } => handle_audit(&cli, action.as_deref(), resource.as_deref(), user.as_deref(), since.as_deref(), until.as_deref(), *limit, *offset).await,
        Command::Chat {
            service,
            prompt,
            max_tokens,
        } => handle_chat(&cli, service, prompt, *max_tokens).await,
        Command::Config(c) => handle_config(&cli, c).await,
    }
}

// ---- handlers -------------------------------------------------------------

async fn handle_auth(cmd: &AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Login {
            api_key,
            gateway_url,
        } => {
            let key = match api_key {
                Some(k) => k.clone(),
                None => {
                    use std::io::{self, BufRead, Write};
                    print!("API key: ");
                    io::stdout().flush()?;
                    let stdin = io::stdin();
                    let mut line = String::new();
                    stdin.lock().read_line(&mut line)?;
                    line.trim().to_string()
                }
            };
            if key.is_empty() {
                return Err(anyhow!("api key cannot be empty"));
            }
            config::save(&config::StoredConfig {
                gateway_url: gateway_url.clone(),
                api_key: Some(key),
            })?;
            ok(&format!("saved credentials for {}", gateway_url));
        }
        AuthCmd::Logout => {
            config::clear()?;
            ok("logged out");
        }
        AuthCmd::Where => {
            println!("{}", config::config_path()?.display());
        }
    }
    Ok(())
}

async fn handle_whoami(cli: &Cli) -> Result<()> {
    let client = resolve_client(cli)?;
    let me = client.whoami().await?;
    if cli.json {
        out_json(cli.json, &me)?;
    } else {
        println!(
            "{}\n  id:    {}\n  email: {}\n  tier:  {}",
            "Authenticated".bold(),
            me.id,
            me.email.as_deref().unwrap_or("(none)"),
            me.tier.as_deref().unwrap_or("(none)"),
        );
    }
    Ok(())
}

async fn handle_doctor(cli: &Cli) -> Result<()> {
    let stored = config::load().ok();
    let url = cli
        .gateway_url
        .clone()
        .or_else(|| stored.as_ref().map(|c| c.gateway_url.clone()))
        .unwrap_or_else(|| "http://localhost:8030".to_string());
    let auth_source = if cli.api_key.is_some() {
        "--api-key flag / MG_API_KEY env"
    } else if stored.as_ref().and_then(|c| c.api_key.clone()).is_some() {
        "~/.mawigateway/config.yaml"
    } else {
        "(none — unauthenticated)"
    };
    println!("gateway URL: {}", url);
    println!("auth source: {}", auth_source);
    println!("config path: {}", config::config_path()?.display());

    // Probe the gateway. Cheap GET that returns 200 if the server is up.
    let client = resolve_client(cli)?;
    match client.whoami().await {
        Ok(me) => ok(&format!("authenticated as {}", me.id)),
        Err(e) => err(&format!("auth probe failed: {}", e)),
    }
    Ok(())
}

async fn handle_providers(cli: &Cli, cmd: &ProvidersCmd) -> Result<()> {
    let client = resolve_client(cli)?;
    match cmd {
        ProvidersCmd::List => {
            let items = client.list_providers().await?;
            if cli.json {
                out_json(true, &items)?;
            } else {
                let rows = items
                    .iter()
                    .map(|p| {
                        vec![
                            p.id.to_string(),
                            p.name.clone(),
                            p.provider_type.clone(),
                            if p.has_api_key { "yes".to_string() } else { "no".to_string() },
                            p.created_at.format("%Y-%m-%d").to_string(),
                        ]
                    })
                    .collect();
                print_table(&["id", "name", "type", "key?", "created"], rows);
            }
        }
        ProvidersCmd::Add {
            name,
            provider_type,
            api_key,
            endpoint,
            api_version,
        } => {
            let body = CreateProvider {
                name: name.clone(),
                provider_type: provider_type.clone(),
                api_key: api_key.clone(),
                api_endpoint: endpoint.clone(),
                api_version: api_version.clone(),
            };
            let p = client.create_provider(&body).await?;
            if cli.json {
                out_json(true, &p)?;
            } else {
                ok(&format!("provider added: {} ({})", p.name, p.id));
            }
        }
        ProvidersCmd::Remove { id } => {
            client.delete_provider(&id.to_string()).await?;
            ok(&format!("provider removed: {}", id));
        }
    }
    Ok(())
}

async fn handle_models(cli: &Cli, cmd: &ModelsCmd) -> Result<()> {
    let client = resolve_client(cli)?;
    match cmd {
        ModelsCmd::List => {
            let items = client.list_models().await?;
            if cli.json {
                out_json(true, &items)?;
            } else {
                let rows = items
                    .iter()
                    .map(|m| {
                        vec![
                            m.id.to_string(),
                            m.name.clone(),
                            m.modality.clone(),
                            m.health_status.clone().unwrap_or_else(|| "?".into()),
                            m.created_at.format("%Y-%m-%d").to_string(),
                        ]
                    })
                    .collect();
                print_table(&["id", "name", "modality", "health", "created"], rows);
            }
        }
        ModelsCmd::Add {
            name,
            provider,
            modality,
            endpoint,
            api_version,
            api_key,
        } => {
            let body = CreateModel {
                name: name.clone(),
                provider: *provider,
                modality: modality.clone(),
                api_endpoint: endpoint.clone(),
                api_version: api_version.clone(),
                api_key: api_key.clone(),
            };
            let m = client.create_model(&body).await?;
            if cli.json {
                out_json(true, &m)?;
            } else {
                ok(&format!("model added: {} ({})", m.name, m.id));
            }
        }
        ModelsCmd::Remove { id } => {
            client.delete_model(&id.to_string()).await?;
            ok(&format!("model removed: {}", id));
        }
    }
    Ok(())
}

async fn handle_services(cli: &Cli, cmd: &ServicesCmd) -> Result<()> {
    let client = resolve_client(cli)?;
    match cmd {
        ServicesCmd::List => {
            let items = client.list_services().await?;
            if cli.json {
                out_json(true, &items)?;
            } else {
                let rows = items
                    .iter()
                    .map(|s| {
                        vec![
                            s.name.clone(),
                            s.service_type.clone().unwrap_or_default(),
                            s.strategy.clone().unwrap_or_default(),
                            s.modality.clone().unwrap_or_default(),
                            s.model_ids.len().to_string(),
                            if s.aliases.is_empty() {
                                "—".to_string()
                            } else {
                                s.aliases.join(", ")
                            },
                        ]
                    })
                    .collect();
                print_table(
                    &["name", "type", "strategy", "modality", "models", "aliases"],
                    rows,
                );
            }
        }
        ServicesCmd::Create {
            name,
            service_type,
            strategy,
            modality,
            models,
            description,
            aliases,
        } => {
            let body = CreateService {
                name: name.clone(),
                service_type: service_type.clone(),
                strategy: Some(strategy.clone()),
                modality: Some(modality.clone()),
                description: description.clone(),
                model_ids: models.clone(),
                aliases: aliases.clone(),
            };
            let s = client.create_service(&body).await?;
            if cli.json {
                out_json(true, &s)?;
            } else {
                ok(&format!("service created: {}", s.name));
                if !s.aliases.is_empty() {
                    println!("  aliases: {}", s.aliases.join(", "));
                }
            }
        }
        ServicesCmd::Update {
            name,
            service_type,
            strategy,
            description,
            planner_model_id,
            system_prompt,
            max_iterations,
            aliases,
            clear_aliases,
        } => {
            // Resolve `aliases` semantics: explicit list wins, then
            // `--clear-aliases` (Some(vec![]) = clear), else None
            // (leave alone).
            let resolved_aliases: Option<Vec<String>> = if *clear_aliases {
                Some(Vec::new())
            } else {
                aliases.clone()
            };
            let body = UpdateService {
                service_type: service_type.clone(),
                description: description.clone(),
                strategy: strategy.clone(),
                guardrails: None,
                pool_type: None,
                planner_model_id: planner_model_id.clone(),
                system_prompt: system_prompt.clone(),
                max_iterations: *max_iterations,
                aliases: resolved_aliases,
            };
            let s = client.update_service(name, &body).await?;
            if cli.json {
                out_json(true, &s)?;
            } else {
                ok(&format!("service updated: {}", s.name));
                if !s.aliases.is_empty() {
                    println!("  aliases: {}", s.aliases.join(", "));
                }
            }
        }
        ServicesCmd::Delete { name } => {
            client.delete_service(name).await?;
            ok(&format!("service deleted: {}", name));
        }
    }
    Ok(())
}

async fn handle_keys(cli: &Cli, cmd: &KeysCmd) -> Result<()> {
    let client = resolve_client(cli)?;
    match cmd {
        KeysCmd::List => {
            let items = client.list_api_keys().await?;
            if cli.json {
                out_json(true, &items)?;
            } else {
                let rows = items
                    .iter()
                    .map(|k| {
                        vec![
                            k.id.to_string(),
                            k.name.clone(),
                            k.prefix.clone(),
                            k.created_at.format("%Y-%m-%d").to_string(),
                            k.expires_at
                                .map(|e| e.format("%Y-%m-%d").to_string())
                                .unwrap_or_else(|| "never".into()),
                            if k.scopes.is_empty() {
                                "admin".to_string()
                            } else {
                                k.scopes.join(", ")
                            },
                        ]
                    })
                    .collect();
                print_table(
                    &["id", "name", "prefix", "created", "expires", "scopes"],
                    rows,
                );
            }
        }
        KeysCmd::Create { name, scopes } => {
            // Empty Vec means "user didn't pass --scopes" → send None
            // so the server defaults to ["admin"]. Non-empty Vec → pass
            // through verbatim and let the server validate it.
            let scopes_arg = if scopes.is_empty() {
                None
            } else {
                Some(scopes.clone())
            };
            let key = client.create_api_key(name, scopes_arg).await?;
            if cli.json {
                out_json(true, &key)?;
            } else {
                ok(&format!("api key created: {}", key.id));
                if !key.scopes.is_empty() {
                    println!("  scopes: {}", key.scopes.join(", "));
                }
                println!();
                println!(
                    "{}",
                    "Copy this key now — it will never be shown again:"
                        .yellow()
                        .bold()
                );
                println!("  {}", key.raw_key.bold());
            }
        }
        KeysCmd::Revoke { id } => {
            client.revoke_api_key(id).await?;
            ok(&format!("api key revoked: {}", id));
        }
    }
    Ok(())
}

async fn handle_mcp(cli: &Cli, cmd: &McpCmd) -> Result<()> {
    let client = resolve_client(cli)?;
    match cmd {
        McpCmd::List => {
            let items = client.list_mcp_servers().await?;
            if cli.json {
                out_json(true, &items)?;
            } else {
                let rows = items
                    .iter()
                    .map(|s| {
                        vec![
                            s.id.to_string(),
                            s.name.clone(),
                            s.server_type.clone(),
                            s.status.clone(),
                            s.image_or_command.clone(),
                        ]
                    })
                    .collect();
                print_table(&["id", "name", "type", "status", "target"], rows);
            }
        }
        McpCmd::Add {
            name,
            server_type,
            image_or_command,
        } => {
            let body = CreateMcpServer {
                name: name.clone(),
                server_type: server_type.as_str().to_string(),
                image_or_command: image_or_command.clone(),
                args: vec![],
                env_vars: serde_json::Map::new(),
            };
            let s = client.create_mcp_server(&body).await?;
            if cli.json {
                out_json(true, &s)?;
            } else {
                ok(&format!("mcp server added: {} ({})", s.name, s.id));
            }
        }
        McpCmd::Connect { id } => {
            let v = client.connect_mcp_server(&id.to_string()).await?;
            if cli.json {
                out_json(true, &v)?;
            } else {
                ok(&format!("connect dispatched for {}", id));
            }
        }
        McpCmd::Remove { id } => {
            client.delete_mcp_server(&id.to_string()).await?;
            ok(&format!("mcp server removed: {}", id));
        }
    }
    Ok(())
}

async fn handle_logs(cli: &Cli, limit: u32) -> Result<()> {
    let client = resolve_client(cli)?;
    let v = client.get_logs(limit).await?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    // Try to render as a table; fall back to raw if shape is unfamiliar.
    let items = v
        .get("items")
        .or(Some(&v))
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    if items.is_empty() {
        println!("(no logs)");
        return Ok(());
    }
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|e| {
            vec![
                json_str(e, "timestamp"),
                json_str(e, "service"),
                json_str(e, "model"),
                json_str(e, "status"),
                json_str(e, "latency_ms"),
            ]
        })
        .collect();
    print_table(&["time", "service", "model", "status", "ms"], rows);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_audit(
    cli: &Cli,
    action: Option<&str>,
    resource: Option<&str>,
    user: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    limit: u32,
    offset: u32,
) -> Result<()> {
    let client = resolve_client(cli)?;
    let v = client
        .get_audit(action, resource, user, since, until, limit, offset)
        .await?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    // Render the page as a table. The shape comes from AuditPage:
    // { items: [...], total, limit, offset }.
    let items = v.get("items").and_then(|x| x.as_array()).cloned().unwrap_or_default();
    let total = v.get("total").and_then(|x| x.as_i64()).unwrap_or(0);
    if items.is_empty() {
        println!("(no entries)");
        return Ok(());
    }
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|e| {
            vec![
                json_str(e, "created_at"),
                json_str(e, "action"),
                json_str(e, "resource"),
                json_str(e, "user_id"),
                json_str(e, "ip_address"),
            ]
        })
        .collect();
    print_table(&["time", "action", "resource", "user", "ip"], rows);
    println!(
        "{} of {} entries (limit {}, offset {})",
        items.len(),
        total,
        limit,
        offset
    );
    Ok(())
}

async fn handle_analytics(cli: &Cli, range: &str) -> Result<()> {
    let client = resolve_client(cli)?;
    let v = client.get_analytics(range).await?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        // Pretty-print top-level keys.
        if let Some(obj) = v.as_object() {
            for (k, val) in obj {
                println!("{}: {}", k.bold(), val);
            }
        } else {
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
    }
    Ok(())
}

async fn handle_chat(
    cli: &Cli,
    service: &str,
    prompt: &str,
    max_tokens: Option<u32>,
) -> Result<()> {
    let client = resolve_client(cli)?;
    let body = ChatRequest {
        service: service.to_string(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: prompt.into(),
        }],
        max_tokens,
        temperature: None,
        stream: false,
    };
    let v = client.chat(&body).await?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&v)?);
        return Ok(());
    }
    // Pull the assistant message out of the OpenAI-shaped response.
    let content = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("(no content)");
    println!("{}", content);
    Ok(())
}

async fn handle_config(cli: &Cli, cmd: &ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::Validate { file } => {
            let text = std::fs::read_to_string(file)
                .with_context(|| format!("read {}", file.display()))?;
            let _: serde_yaml::Value =
                serde_yaml::from_str(&text).context("parse YAML")?;
            ok(&format!("valid YAML: {}", file.display()));
        }
        ConfigCmd::Apply { file } => {
            let client = resolve_client(cli)?;
            let text = std::fs::read_to_string(file)
                .with_context(|| format!("read {}", file.display()))?;
            let v = client.apply_config(&text).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                ok(&format!("applied: {}", file.display()));
            }
        }
    }
    Ok(())
}

fn json_str(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(other) => other.to_string(),
        None => "".into(),
    }
}

// ---------------------------------------------------------------------------
// Re-exports so config module can read these structs.
// ---------------------------------------------------------------------------

pub use config::StoredConfig;

#[derive(Debug, Serialize, Deserialize)]
struct _DeadCode; // suppress unused-import warning for serde when feature gates change
