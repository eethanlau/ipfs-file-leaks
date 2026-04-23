//! gRPC client for the TTL-gated key server.

use std::time::Duration;

use tonic::transport::{Endpoint, Uri};

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
        encryption_key: &[u8],
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
            encryption_key: encryption_key.to_vec(),
            ttl_seconds: ttl.as_secs() as i64,
            is_replication: false,
        });

        client.register_key(request).await?;
        Ok(())
    }

    pub async fn fetch(&self, cid: &str) -> Result<Vec<u8>, KeyClientError> {
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
        if response.success {
            Ok(response.encryption_key)
        } else {
            Err(KeyClientError::ServerRejected(response.message))
        }
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
}
