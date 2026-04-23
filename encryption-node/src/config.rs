//! Runtime configuration collected by binaries and passed into the library.

use std::time::Duration;

use tonic::transport::Uri;
use url::Url;

/// Endpoints and policy knobs for a single run of the client.
///
/// The library never reads config from the environment or disk; binaries
/// construct this and pass it in.
pub struct Config {
    pub ipfs_url: Url,
    pub key_server_url: Uri,
    pub default_ttl: Duration,
}

impl Config {
    pub fn default_localhost() -> Self {
        Self {
            ipfs_url: Url::parse("http://127.0.0.1:5001/").expect("static URL parses"),
            key_server_url: Uri::from_static("http://127.0.0.1:50051"),
            default_ttl: Duration::from_secs(86_400),
        }
    }
}
