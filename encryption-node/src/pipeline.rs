//! Orchestration of the publish/retrieve flow over IPFS + the key server.
//!
//! Single composition point for `crypto`, `ipfs`, and `key_client`; binaries
//! drive the system through this module.

use std::time::{Duration, Instant};

use crate::config::Config;
use crate::crypto::{self, SecretKey};
use crate::error::Error;
use crate::ipfs::IpfsClient;
use crate::key_client::KeyClient;

/// Per-step latency breakdown for a `publish` call.
#[derive(Debug, Clone, Copy)]
pub struct PublishTimings {
    pub key_gen: Duration,
    pub encrypt: Duration,
    pub ipfs_add: Duration,
    pub key_register: Duration,
}

/// Result of a successful `publish`: the IPFS CID, the TTL the key server
/// was asked to honor, and per-step timings.
pub struct PublishOutcome {
    pub cid: String,
    pub ttl: Duration,
    pub timings: PublishTimings,
}

/// Per-step latency breakdown for a `retrieve` call.
#[derive(Debug, Clone, Copy)]
pub struct RetrieveTimings {
    pub key_fetch: Duration,
    pub ipfs_cat: Duration,
    pub decrypt: Duration,
}

/// Result of a successful `retrieve`: the recovered plaintext and per-step timings.
pub struct RetrieveOutcome {
    pub plaintext: Vec<u8>,
    pub timings: RetrieveTimings,
}

pub struct Pipeline {
    ipfs: IpfsClient,
    keys: KeyClient,
    default_ttl: Duration,
}

impl Pipeline {
    pub fn new(config: Config) -> Self {
        Self {
            ipfs: IpfsClient::new(config.ipfs_url),
            keys: KeyClient::new(config.key_server_url),
            default_ttl: config.default_ttl,
        }
    }

    /// TTL the binary should pass to `publish` when the operator hasn't
    /// supplied one explicitly.
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// Encrypts `plaintext` under a fresh key, uploads the envelope to IPFS,
    /// and registers the key with the key server under that CID for `ttl`.
    #[tracing::instrument(skip(self, plaintext), fields(plaintext_len = plaintext.len()))]
    pub async fn publish(&self, plaintext: &[u8], ttl: Duration) -> Result<PublishOutcome, Error> {
        let t0 = Instant::now();
        let key = SecretKey::generate();
        let t1 = Instant::now();
        let envelope = crypto::encrypt(plaintext, &key)?;
        let t2 = Instant::now();
        let cid = self.ipfs.add(envelope).await?;
        let t3 = Instant::now();
        self.keys.register(&cid, &key, ttl).await?;
        let t4 = Instant::now();

        Ok(PublishOutcome {
            cid,
            ttl,
            timings: PublishTimings {
                key_gen: t1.duration_since(t0),
                encrypt: t2.duration_since(t1),
                ipfs_add: t3.duration_since(t2),
                key_register: t4.duration_since(t3),
            },
        })
    }

    /// Fetches the key for `cid` from the key server, downloads the envelope
    /// from IPFS, and decrypts. Returns the recovered plaintext and timings.
    #[tracing::instrument(skip(self))]
    pub async fn retrieve(&self, cid: &str) -> Result<RetrieveOutcome, Error> {
        let t0 = Instant::now();
        let key = self.keys.fetch(cid).await?;
        let t1 = Instant::now();
        let envelope = self.ipfs.cat(cid).await?;
        let t2 = Instant::now();
        let plaintext = crypto::decrypt(&envelope, &key)?;
        let t3 = Instant::now();

        Ok(RetrieveOutcome {
            plaintext,
            timings: RetrieveTimings {
                key_fetch: t1.duration_since(t0),
                ipfs_cat: t2.duration_since(t1),
                decrypt: t3.duration_since(t2),
            },
        })
    }
}
