//! Orchestration of the publish/retrieve flow over IPFS + the key server.
//!
//! Single composition point for `crypto`, `ipfs`, and `key_client`; binaries
//! drive the system through this module.

use std::time::Duration;

use crate::config::Config;
use crate::crypto::{self, SecretKey};
use crate::error::Error;
use crate::ipfs::IpfsClient;
use crate::key_client::KeyClient;

/// Result of a successful `publish`: the IPFS CID for the encrypted envelope
/// and the TTL the key server was asked to honor.
pub struct PublishOutcome {
    pub cid: String,
    pub ttl: Duration,
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
    pub async fn publish(&self, plaintext: &[u8], ttl: Duration) -> Result<PublishOutcome, Error> {
        let key = SecretKey::generate();
        let envelope = crypto::encrypt(plaintext, &key)?;
        let cid = self.ipfs.add(envelope).await?;
        self.keys.register(&cid, &key, ttl).await?;
        Ok(PublishOutcome { cid, ttl })
    }

    /// Fetches the key for `cid` from the key server, downloads the envelope
    /// from IPFS, and decrypts. Returns the original plaintext bytes.
    pub async fn retrieve(&self, cid: &str) -> Result<Vec<u8>, Error> {
        let key = self.keys.fetch(cid).await?;
        let envelope = self.ipfs.cat(cid).await?;
        let plaintext = crypto::decrypt(&envelope, &key)?;
        Ok(plaintext)
    }
}
