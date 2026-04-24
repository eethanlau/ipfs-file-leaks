//! gRPC client for the TTL-gated key server.

use std::time::Duration;

use tonic::transport::{Endpoint, Uri};

use crate::crypto::{CryptoError, SecretKey};

pub(crate) mod pb {
    tonic::include_proto!("key_service");
}

use pb::key_service_client::KeyServiceClient;
use pb::{GetKeyRequest, RegisterKeyRequest};

pub struct KeyClient {
    endpoint: Endpoint,
}

impl KeyClient {
    pub fn new(uri: Uri) -> Self {
        Self {
            endpoint: Endpoint::from(uri),
        }
    }

    pub async fn register(
        &self,
        cid: &str,
        key: &SecretKey,
        ttl: Duration,
    ) -> Result<(), KeyClientError> {
        let channel = self
            .endpoint
            .connect()
            .await
            .map_err(KeyClientError::Connect)?;
        let mut client = KeyServiceClient::new(channel);

        let request = tonic::Request::new(RegisterKeyRequest {
            cid: cid.to_string(),
            encryption_key: key.expose().to_vec(),
            ttl_seconds: ttl.as_secs() as i64,
            is_replication: false,
        });

        client.register_key(request).await?;
        Ok(())
    }

    pub async fn fetch(&self, cid: &str) -> Result<SecretKey, KeyClientError> {
        let channel = self
            .endpoint
            .connect()
            .await
            .map_err(KeyClientError::Connect)?;
        let mut client = KeyServiceClient::new(channel);

        let request = tonic::Request::new(GetKeyRequest {
            cid: cid.to_string(),
        });

        let response = client.get_key(request).await?.into_inner();
        if !response.success {
            return Err(KeyClientError::ServerRejected(response.message));
        }
        Ok(SecretKey::from_bytes(&response.encryption_key)?)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum KeyClientError {
    #[error("failed to connect to key server")]
    Connect(#[source] tonic::transport::Error),
    #[error("RPC failure")]
    Rpc(#[from] tonic::Status),
    #[error("server rejected request: {0}")]
    ServerRejected(String),
    #[error("server returned malformed key material")]
    InvalidKeyMaterial(#[from] CryptoError),
}
