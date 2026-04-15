use ipfs_api_backend_hyper::{IpfsApi, IpfsClient, TryFromUri};
use std::io::Cursor;

// Function for uploading ciphertext to IPFS node
pub async fn upload_to_ipfs(data: Vec<u8>) -> Result<String, String> {
    // 1. Initialize the client
    let client = IpfsClient::from_str("http://127.0.0.1:5001")
        .map_err(|e| format!("Failed to create IPFS client: {}", e))?;
    
    // 2. Wrap the Vec<u8> directly in a Cursor 
    let cursor = Cursor::new(data);

    // 3. Call 'add' method to upload the ciphertext and get cid response
    match client.add(cursor).await {
        Ok(response) => Ok(response.hash),
        Err(e) => Err(format!("Error uploading to IPFS: {}", e)),
    }
}