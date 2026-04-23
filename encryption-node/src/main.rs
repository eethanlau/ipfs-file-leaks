use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use encryption_node::config::Config;
use encryption_node::crypto;
use encryption_node::ipfs::IpfsClient;
use encryption_node::key_client::KeyClient;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: cargo run <publish|retrieve> <file_path|cid>");
        return;
    }

    let command = &args[1];
    let target = &args[2];

    let cfg = Config::default_localhost();

    match command.as_str() {
        "publish" => {
            println!("Starting upload flow for file: {}", target);
            let file_contents =
                read_file(target.to_string()).expect("Failed to read file contents");
            let crypto_key = crypto::generate_symmetric_key();
            let ciphertext =
                crypto::encrypt_file(&file_contents, &crypto_key).expect("Failed to encrypt file");

            let ipfs = IpfsClient::new(cfg.ipfs_url.clone());
            let cid = ipfs
                .add(ciphertext)
                .await
                .expect("Failed to upload file to IPFS node");

            let keys = KeyClient::new(cfg.key_server_url.clone());
            keys.register(&cid, &crypto_key, cfg.default_ttl)
                .await
                .expect("Failed to register key");
            println!("File published, CID: {}", cid);
        }
        "retrieve" => {
            println!("Starting download flow for CID: {}", target);

            let ipfs = IpfsClient::new(cfg.ipfs_url.clone());
            let ciphertext = ipfs.cat(target).await.expect("Failed to download file");

            let keys = KeyClient::new(cfg.key_server_url.clone());
            let crypto_key = keys
                .fetch(target)
                .await
                .expect("failed to contact key server to retrieve key");

            let plaintext =
                crypto::decrypt_file(&ciphertext, &crypto_key).expect("Failed to decrypt file");

            std::fs::write("decrypted_output", plaintext).expect("Failed to save decrypted file");
            println!("File retrieved and saved as 'decrypted_output'");
        }
        _ => {
            println!("Unknown command. Please use 'publish' or 'retrieve' in order to either publish or retrieve the file/s.");
        }
    }
}

pub fn read_file(target: String) -> io::Result<Vec<u8>> {
    let mut f = File::open(target)?;

    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}
