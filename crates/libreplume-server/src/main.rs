use clap::{Parser, Subcommand};
use libreplume_server::service::{AppState, Config};
use libreplume_server::store::Store;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "libreplume", version, about = "Grammar, spelling, conjugation and style checker (French & English)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Directory where settings, dictionary and themes are stored.
    #[arg(long, env = "LIBREPLUME_DATA_DIR", default_value = "./data", global = true)]
    data_dir: PathBuf,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server (web interface, REST API and MCP endpoint). Default command.
    Serve {
        #[arg(long, env = "LIBREPLUME_HOST", default_value = "0.0.0.0")]
        host: String,
        #[arg(long, short, env = "LIBREPLUME_PORT", default_value_t = 8080)]
        port: u16,
        /// Bearer token required for changes and for /mcp. Strongly advised on public servers.
        #[arg(long, env = "LIBREPLUME_TOKEN", hide_env_values = true)]
        token: Option<String>,
        /// Maximum number of characters per check.
        #[arg(long, env = "LIBREPLUME_MAX_TEXT_CHARS", default_value_t = 200_000)]
        max_text_chars: usize,
    },
    /// Check files (or stdin with "-") and print the issues.
    Check {
        files: Vec<PathBuf>,
        #[arg(long, short)]
        language: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Serve {
        host: std::env::var("LIBREPLUME_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
        port: std::env::var("LIBREPLUME_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080),
        token: std::env::var("LIBREPLUME_TOKEN").ok().filter(|t| !t.is_empty()),
        max_text_chars: 200_000,
    });
    // Logs go to stderr so that stdout stays clean for the MCP stdio transport.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("LIBREPLUME_LOG")
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    match command {
        Command::Serve { host, port, token, max_text_chars } => {
            let store = Store::open(Some(&cli.data_dir))?;
            let started = std::time::Instant::now();
            tokio::task::spawn_blocking(libreplume_core::resources::preload).await?;
            tracing::info!("dictionaries loaded in {:?}", started.elapsed());
            let token = token.filter(|t| !t.is_empty());
            if token.is_none() {
                tracing::warn!("no LIBREPLUME_TOKEN set: anyone who can reach the server can change its settings");
            }
            let state = AppState { store: Arc::new(store), config: Arc::new(Config { token, max_text_chars }) };
            let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await?;
            tracing::info!("LibrePlume {} listening on http://{}", libreplume_core::VERSION, listener.local_addr()?);
            axum::serve(listener, libreplume_server::app(state)).with_graceful_shutdown(shutdown()).await?;
        }
        Command::Check { files, language } => {
            let store = Store::open(Some(&cli.data_dir)).unwrap_or_else(|_| Store::in_memory());
            let overrides = libreplume_server::store::RequestOverrides {
                language: language.map(|l| l.parse()).transpose()?,
                ..Default::default()
            };
            let options = store.check_options(&overrides);
            let mut found = 0;
            let inputs = if files.is_empty() { vec![PathBuf::from("-")] } else { files };
            for file in inputs {
                let text = if file.as_os_str() == "-" {
                    std::io::read_to_string(std::io::stdin())?
                } else {
                    std::fs::read_to_string(&file)?
                };
                let result = libreplume_core::check(&text, &options);
                for m in &result.matches {
                    let line = text[..m.byte_start].matches('\n').count() + 1;
                    let suggestion = m.replacements.first().map(|r| format!(" → {r:?}")).unwrap_or_default();
                    println!("{}:{line}: [{}] {:?}{suggestion}: {}", file.display(), m.rule_id, m.text, m.message);
                }
                found += result.matches.len();
            }
            if found > 0 {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

async fn shutdown() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
