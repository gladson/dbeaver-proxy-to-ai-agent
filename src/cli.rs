use clap::{Parser, Subcommand};

use crate::config::Config;

#[derive(Parser)]
#[command(
    name = "dbeaver-proxy",
    version,
    about = "DBeaver AI proxy — translates OpenAI Responses API to any compatible backend"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive first-run setup wizard
    Init(InitArgs),
    /// Start the proxy server
    Start(StartArgs),
}

#[derive(clap::Args, Clone)]
pub struct InitArgs {
    /// Backend base URL (e.g., https://api.openai.com/v1)
    #[arg(long, env = "DBEAVER_PROXY_BASE_URL")]
    pub base_url: Option<String>,

    /// API key for the backend
    #[arg(long, env = "DBEAVER_PROXY_API_KEY")]
    pub api_key: Option<String>,

    /// Default model to use (e.g., gpt-4o)
    #[arg(long, env = "DBEAVER_PROXY_MODEL")]
    pub model: Option<String>,

    /// Path to config file
    #[arg(long, default_value = "dbeaver-proxy.toml")]
    pub config_path: String,
}

#[derive(clap::Args, Clone)]
pub struct StartArgs {
    /// Path to config file
    #[arg(long, default_value = "dbeaver-proxy.toml")]
    pub config_path: String,
}

pub async fn cmd_init(args: InitArgs) {
    println!("{}", BANNER);
    println!("\n🔧 First-time setup — let's configure your proxy.");
    println!("Press Ctrl+C at any time to cancel.\n");

    use dialoguer::{Input, Password};

    let base_url: String = if let Some(url) = args.base_url {
        url
    } else {
        Input::new()
            .with_prompt("Backend Base URL")
            .default("https://api.openai.com/v1".to_string())
            .interact_text()
            .unwrap_or_default()
    };

    let api_key: String = if let Some(key) = args.api_key {
        key
    } else {
        Password::new()
            .with_prompt("API Key")
            .with_confirmation("Confirm API Key", "Keys don't match")
            .interact()
            .unwrap_or_default()
    };

    let model: String = if let Some(m) = args.model {
        m
    } else {
        Input::new()
            .with_prompt("Default Model")
            .default("gpt-4o".to_string())
            .interact_text()
            .unwrap_or_default()
    };

    let config = Config {
        base_url,
        api_key,
        model,
    };

    match config.write(&args.config_path) {
        Ok(path) => println!("\n✅ Configuration saved to: {}", path),
        Err(e) => eprintln!("\n❌ Failed to save config: {}", e),
    }
}

pub async fn cmd_start(args: StartArgs) {
    match Config::load(&args.config_path) {
        Ok(config) => {
            let proxy_port = std::env::var("PORT").unwrap_or_else(|_| "60916".to_string());
            let proxy_host = std::env::var("HOST").unwrap_or_else(|_| "localhost".to_string());
            let proxy_url = format!("http://{}:{}/v1/", proxy_host, proxy_port);

            println!("✅ Configuration loaded from: {}", args.config_path);
            println!("   Proxy URL:  {}", proxy_url);
            println!("   Backend:    {}", config.base_url);
            println!("   Model:      {}", config.model);
            println!("\n🚀 Starting proxy server...");
            println!("   Configure DBeaver with:");
            println!("   - Base URL: {}", proxy_url);
            println!("   - API Key:  {}", config.api_key);
            println!("   - Model:    {}", config.model);

            crate::router::run(config).await;
        }
        Err(_) => {
            eprintln!("❌ No config file found at: {}", args.config_path);
            eprintln!();
            eprintln!("   Run `dbeaver-proxy init` to create one interactively.");
            eprintln!("   Or run `dbeaver-proxy --help` to see all options.");
        }
    }
}

const BANNER: &str = r#"
╔══════════════════════════════════════════╗
║         DBeaver Proxy — Rust            ║
║     OpenAI Responses → Chat Completions  ║
║                                          ║
║         Created by Gladson               ║
║   gladsonbrito@gmail.com                 ║
╚══════════════════════════════════════════╝
"#;
