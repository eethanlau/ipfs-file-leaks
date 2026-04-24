//! Crate-level error type aggregating per-module errors.

use crate::crypto::CryptoError;
use crate::ipfs::IpfsError;
use crate::key_client::KeyClientError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Ipfs(#[from] IpfsError),
    #[error(transparent)]
    Key(#[from] KeyClientError),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
