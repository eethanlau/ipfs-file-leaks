//! CLI entry point for the encryption-node client.

use std::path::PathBuf;
use std::time::Duration;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use tonic::transport::Uri;
use tracing_subscriber::EnvFilter;
use url::Url;

use encryption_node::config::Config;
use encryption_node::pipeline::Pipeline;

#[derive(Parser)]
#[command(
    version,
    about = "Publish and retrieve TTL-gated encrypted files on IPFS"
)]
struct Cli {
    /// IPFS HTTP API endpoint (Kubo-compatible)
    #[arg(long, env = "IPFS_URL", default_value = "http://127.0.0.1:5001/")]
    ipfs_url: Url,

    /// Key server gRPC endpoint
    #[arg(long, env = "KEY_SERVER_URL", default_value = "http://127.0.0.1:50051")]
    key_server_url: Uri,

    /// TTL the key server holds the encryption key for, e.g. "24h" or "30m"
    #[arg(long, env = "TTL", default_value = "24h", value_parser = parse_ttl)]
    ttl: Duration,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Encrypt a file, upload the envelope to IPFS, and register its key
    Publish { path: PathBuf },
    /// Fetch the key, download the envelope from IPFS, and decrypt
    Retrieve { cid: String },
}

fn parse_ttl(input: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(input)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let pipeline = Pipeline::new(Config {
        ipfs_url: cli.ipfs_url,
        key_server_url: cli.key_server_url,
        default_ttl: cli.ttl,
    });

    match cli.command {
        Command::Publish { path } => {
            let plaintext = std::fs::read(&path)?;
            let outcome = pipeline.publish(&plaintext, pipeline.default_ttl()).await?;
            println!(
                "published cid={} ttl={}",
                outcome.cid,
                humantime::format_duration(outcome.ttl)
            );
        }
        Command::Retrieve { cid } => {
            let plaintext = pipeline.retrieve(&cid).await?;
            std::fs::write("decrypted_output", &plaintext)?;
            println!(
                "retrieved cid={} bytes={} -> decrypted_output",
                cid,
                plaintext.len()
            );
        }
    }

    Ok(())
}
