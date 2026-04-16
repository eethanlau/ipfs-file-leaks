use std::env; // imports
use std::io;
use std::io::prelude::*;
use std::fs::File;
// Default TTL is 24 hours; config for ttl for keys
const DEFAULT_TTL_SECONDS: i64 = 86400;

// files to link
mod crypto;
mod client;
mod ipfs;
mod audit;

#[tokio::main]
async fn main() {
    // Rust CLI for different functions based on the arguments involved
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: cargo run <publish|retrieve> <file_path|cid>");
        return;
    }

    // Save the arguments properly
    let command = &args[1];
    let target = &args[2];

    match command.as_str() {
        "publish" => {
            println!("Starting upload flow for file: {}", target.to_string());
            // Need to pass in the actual raw bytes of the target file
            let file_contents = read_file(target.to_string()).expect("Failed to read file contents");
            // 1. publish file
            // 1.1 Use crypto.rs to generate the symmetric key
            let crypto_key = crypto::generate_symmetric_key();
            let ciphertext = crypto::encrypt_file(&file_contents, &crypto_key).expect("Failed to encrypt file");
        
            // 1.2. Upload file to ciphertext to IPFS and get CID (content identifier)
            let cid = ipfs::upload_to_ipfs(ciphertext).await.expect("Failed to upload file to IPFS node");

            // 1.3. Register the key after uploading the cipher text and getting cid
            client::register_key(cid.clone(), crypto_key.to_vec(), DEFAULT_TTL_SECONDS).await.expect("Failed to register key");
            println!("File published, CID: {}", cid);

        },
        "retrieve" => {
            println!("Starting download flow for CID: {}", target);
            // 2. retrieve file

            // 2.1. let ciphertext = ipfs::download(target);
            // Download file with corresponding CID
            let ciphertext = ipfs::download_from_ipfs(target).await.expect("Failed to download file");

            // 2.2. Contact go server with the client to get the key
            let crypto_key = client::get_key(target.to_string()).await.expect("failed to contact go server to retrieve key");

            // 2.3. Decrypt the file with the key
            let plaintext = crypto::decrypt_file(&ciphertext, &crypto_key).expect("Failed to decrypt file");

            // 2.4. Save the decrypted file to disk
            std::fs::write("decrypted_output", plaintext).expect("Failed to save decrypted file");
            println!("File retrieved and saved as 'decrypted_output'");
        },
        _ => {
            // default case
            println!("Unknown command. Please use 'publish' or 'retrieve' in order to either publish or retrieve the file/s.");
        }
    }
}

// Function for reading file contents from file path
pub fn read_file(target: String) -> io::Result<Vec<u8>> {
    let mut f = File::open(target)?;

    let mut buffer = Vec::new();
    // read the whole file into the buffer and return its contents
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}