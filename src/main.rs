use clap::Parser;
use tracing_subscriber::EnvFilter;

mod cli;
mod client;
mod config;
mod handlers;
mod metrics;
mod models;
mod router;
mod sse;
mod translation;

#[tokio::main]
async fn main() {
    // Initialize tracing (text format by default, JSON if LOG_FORMAT=json)
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".to_string());

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if log_format == "json" {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Init(args) => cli::cmd_init(args).await,
        cli::Commands::Start(args) => cli::cmd_start(args).await,
    }
}
