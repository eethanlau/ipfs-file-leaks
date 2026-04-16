use reqwest::multipart;

// Upload ciphertext to the local IPFS Kubo node via its HTTP API
pub async fn upload_to_ipfs(data: Vec<u8>) -> Result<String, String> {
    let client = reqwest::Client::new();

    // Wrap the raw bytes as a multipart file part
    let part = multipart::Part::bytes(data).file_name("file");
    let form = multipart::Form::new().part("file", part);

    // POST to the IPFS HTTP API add endpoint
    let response = client
        .post("http://127.0.0.1:5001/api/v0/add")
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to reach IPFS node: {}", e))?;

    // IPFS returns a JSON object with a "Hash" field containing the CID
    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse IPFS response: {}", e))?;

    json["Hash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No Hash found in IPFS response".to_string())
}

// Download ciphertext from IPFS by CID
pub async fn download_from_ipfs(cid: &str) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::new();

    let url = format!("http://127.0.0.1:5001/api/v0/cat?arg={}", cid);

    let response = client
        .post(&url) // IPFS API uses POST for /cat
        .send()
        .await
        .map_err(|e| format!("Failed to reach IPFS node: {}", e))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(bytes.to_vec())
}
