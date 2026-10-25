#![forbid(unsafe_code)]

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use yesrss::config::Config;
use yesrss::state::AppState;
use yesrss::{auth, db, error, feeds, opml, scheduler, web};

#[derive(Parser)]
#[command(name = "yesrss", version, about = "A small, safe, self-hosted feed reader")]
struct Cli {
    /// Path to the SQLite database file.
    #[arg(long, env = "YESRSS_DB", default_value = "yesrss.db", global = true)]
    db: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the web server and background feed fetcher.
    Serve(Config),
    /// Manage users.
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Import an OPML file of subscriptions for a user.
    ImportOpml { username: String, file: PathBuf },
}

#[derive(Subcommand)]
enum UserCommand {
    /// Create a user. Reads the password from YESRSS_PASSWORD or prompts for it.
    Add { username: String },
    /// Set a user's password and sign out all of their sessions.
    Passwd { username: String },
    /// List users.
    List,
    /// Delete a user and all of their subscriptions.
    Delete { username: String },
}

fn read_password() -> anyhow::Result<String> {
    if let Ok(p) = std::env::var("YESRSS_PASSWORD") {
        return Ok(p);
    }
    if !std::io::stdin().is_terminal() {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        return Ok(line.trim_end_matches(['\r', '\n']).to_owned());
    }
    let p = rpassword::prompt_password("Password: ")?;
    if p != rpassword::prompt_password("Repeat password: ")? {
        bail!("passwords do not match");
    }
    Ok(p)
}

async fn user_id(db: &sqlx::SqlitePool, username: &str) -> anyhow::Result<i64> {
    sqlx::query_scalar("SELECT id FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(db)
        .await?
        .with_context(|| format!("no such user: {username}"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tower_http=warn")))
        .init();
    let cli = Cli::parse();
    let db = db::connect(&cli.db)
        .await
        .with_context(|| format!("opening database {}", cli.db.display()))?;

    match cli.command {
        Command::Serve(cfg) => {
            let listen = cfg.listen;
            if !cfg.allow_private_networks {
                tracing::info!("fetching from private networks is disabled (see --allow-private-networks)");
            }
            let state = AppState::new(db, cfg)?;
            scheduler::spawn(state.clone());
            let listener = tokio::net::TcpListener::bind(listen)
                .await
                .with_context(|| format!("binding {listen}"))?;
            tracing::info!("yesrss listening on http://{listen}");
            axum::serve(listener, web::app(state))
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        }
        Command::User { command } => match command {
            UserCommand::Add { username } => {
                let password = read_password()?;
                let user = auth::create_user(&db, &username, &password).await.map_err(anyhow_err)?;
                println!("created user {} (id {})", user.username, user.id);
            }
            UserCommand::Passwd { username } => {
                let id = user_id(&db, &username).await?;
                let password = read_password()?;
                auth::set_password(&db, id, &password).await.map_err(anyhow_err)?;
                println!("password updated for {username}");
            }
            UserCommand::List => {
                let users: Vec<(i64, String)> = sqlx::query_as("SELECT id, username FROM users ORDER BY id")
                    .fetch_all(&db)
                    .await?;
                for (id, name) in users {
                    println!("{id}\t{name}");
                }
            }
            UserCommand::Delete { username } => {
                let id = user_id(&db, &username).await?;
                sqlx::query("DELETE FROM entries WHERE user_id = ?").bind(id).execute(&db).await?;
                sqlx::query("DELETE FROM users WHERE id = ?").bind(id).execute(&db).await?;
                println!("deleted {username}");
            }
        },
        Command::ImportOpml { username, file } => {
            let id = user_id(&db, &username).await?;
            let xml = std::fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            let items = opml::parse(&xml).map_err(anyhow_err)?;
            let state = AppState::new(db, Config::default())?;
            let (added, skipped) = feeds::import_opml(&state, id, items).await.map_err(anyhow_err)?;
            println!("imported {added} feeds ({skipped} skipped); they will be fetched when the server runs");
        }
    }
    Ok(())
}

fn anyhow_err(e: error::AppError) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {}
        _ = term => {}
    }
    tracing::info!("shutting down");
}
