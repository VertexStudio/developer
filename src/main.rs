use anyhow::Result;
use clap::Parser;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

pub mod counter;
pub mod developer;

#[derive(Parser)]
#[command(name = "developer")]
#[command(about = "A developer MCP server")]
#[command(version)]
struct Cli {
    // No commands for now, just basic CLI structure
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting MCP server");

    // Create an instance of our developer service
    let service = developer::Developer::new()
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("serving error: {:?}", e);
        })?;

    service.waiting().await?;
    Ok(())
}
