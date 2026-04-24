//! TTL experiment harness.
//!
//! Drives the publish/retrieve flow against a running IPFS node and key
//! server, measures wall-clock latencies, and verifies that retrieval
//! fails after the TTL elapses. Emits CSV to stdout; tracing goes to stderr.
//!
//! This harness defines the runtime contract the testbed must satisfy:
//! a Kubo-compatible IPFS HTTP API at `--ipfs-url`, the key server's gRPC
//! service at `--key-server-url`, and the proto-defined behaviors
//! (RegisterKey honors `ttl_seconds`; GetKey rejects expired entries).
//!
//! Note: imports `KeyClientError` to label post-TTL outcomes precisely; the
//! usual "binaries don't reach into key_client" rule is relaxed here because
//! interpreting failure modes is the harness's job.

use std::time::{Duration, Instant};

use clap::Parser;
use color_eyre::eyre::{bail, Result};
use rand::rngs::OsRng;
use rand::RngCore;
use tonic::transport::Uri;
use tracing_subscriber::EnvFilter;
use url::Url;

use encryption_node::config::Config;
use encryption_node::error::Error;
use encryption_node::key_client::KeyClientError;
use encryption_node::pipeline::Pipeline;

#[derive(Parser)]
#[command(version, about = "TTL leak experiment harness")]
struct Cli {
    #[arg(long, env = "IPFS_URL", default_value = "http://127.0.0.1:5001/")]
    ipfs_url: Url,

    #[arg(long, env = "KEY_SERVER_URL", default_value = "http://127.0.0.1:50051")]
    key_server_url: Uri,

    /// Plaintext payload size in bytes
    #[arg(long, default_value_t = 4096)]
    payload_size: usize,

    /// TTL the key server is asked to honor, e.g. "10s" or "2m"
    #[arg(long, default_value = "10s", value_parser = parse_duration)]
    ttl: Duration,

    /// Slack added past the TTL before the post-TTL retrieve attempt
    #[arg(long, default_value = "2s", value_parser = parse_duration)]
    margin: Duration,

    /// Number of publish/retrieve iterations to record
    #[arg(long, default_value_t = 1)]
    iterations: u32,
}

fn parse_duration(input: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(input)
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let pipeline = Pipeline::new(Config {
        ipfs_url: cli.ipfs_url,
        key_server_url: cli.key_server_url,
        default_ttl: cli.ttl,
    });

    println!(
        "iteration,payload_bytes,publish_ms,retrieve_pre_ms,retrieve_post_ms,retrieve_post_outcome"
    );

    let mut leaks = 0u32;
    for iteration in 0..cli.iterations {
        let payload = random_bytes(cli.payload_size);

        let publish_start = Instant::now();
        let outcome = pipeline.publish(&payload, cli.ttl).await?;
        let publish_ms = publish_start.elapsed().as_millis();

        let pre_start = Instant::now();
        let recovered = pipeline.retrieve(&outcome.cid).await?;
        let pre_ms = pre_start.elapsed().as_millis();
        if recovered != payload {
            bail!(
                "pre-TTL retrieve returned mismatched bytes for cid {}",
                outcome.cid
            );
        }

        tokio::time::sleep(cli.ttl + cli.margin).await;

        let post_start = Instant::now();
        let post_result = pipeline.retrieve(&outcome.cid).await;
        let post_ms = post_start.elapsed().as_millis();
        let post_label = label(&post_result);
        if post_result.is_ok() {
            leaks += 1;
        }

        println!(
            "{},{},{},{},{},{}",
            iteration, cli.payload_size, publish_ms, pre_ms, post_ms, post_label
        );
    }

    if leaks > 0 {
        bail!("{leaks} iteration(s) leaked: post-TTL retrieve succeeded");
    }
    Ok(())
}

fn random_bytes(size: usize) -> Vec<u8> {
    let mut buf = vec![0u8; size];
    OsRng.fill_bytes(&mut buf);
    buf
}

fn label(result: &Result<Vec<u8>, Error>) -> &'static str {
    match result {
        Ok(_) => "leak",
        Err(Error::Key(KeyClientError::ServerRejected(_))) => "key_rejected",
        Err(Error::Key(KeyClientError::Connect(_))) => "key_connect_error",
        Err(Error::Key(KeyClientError::Rpc(_))) => "key_rpc_error",
        Err(Error::Key(KeyClientError::InvalidKeyMaterial(_))) => "key_invalid_material",
        Err(Error::Ipfs(_)) => "ipfs_error",
        Err(Error::Crypto(_)) => "crypto_error",
        Err(Error::Io(_)) => "io_error",
    }
}
