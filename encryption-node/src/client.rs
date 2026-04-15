// load rust code from build.rs; generates from your key_service.proto file
pub mod pb {
    tonic::include_proto!("key_service"); // key_service proto
}

use pb::key_service_client::KeyServiceClient;
use pb::{RegisterKeyRequest, GetKeyRequest};

pub async fn register_key(cid: String, encryption_key: Vec<u8>, ttl_seconds: i64) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Connect to the Go server and port
    let mut client = KeyServiceClient::connect("http://127.0.0.1:50051").await?;

    // 2. Prepare register key request using proto definition fields
    let request = tonic::Request::new(RegisterKeyRequest {
        cid,
        encryption_key,
        ttl_seconds,
        is_replication: false,
    });

    // 3. Make gRPC call
    let response = client.register_key(request).await?;

    println!("Go Server Response: {:?}", response.into_inner().message);
    Ok(())
}

pub async fn get_key(cid: String) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut client = KeyServiceClient::connect("http://127.0.0.1:50051").await?;

    let request = tonic::Request::new(GetKeyRequest {
        cid,
    });

    let response = client.get_key(request).await?;
    let response_data = response.into_inner();

    // Get the encryption key and return it to decrypt the ciphertext into the file's contents
    if response_data.success {
        Ok(response_data.encryption_key)
    } else {
        Err(format!("Error from server: {}", response_data.message).into())
    }
}