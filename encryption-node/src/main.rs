use std::env; // imports

// Default TTL is 24 hours; config for ttl for keys
const DEFAULT_TTL_SECONDS: i64 = 86400;

// files to link
mod crypto;
mod client;
mod ipfs;
mod audit;

pub fn main() {
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
            println!("Starting upload flow for file: {}", target);
            // 1. publish file
            // 1.1 Use crypto.rs to generate the symmetric key
            Let crypto_key = crypto::generate_key();
            let ciphertext = crypto::encrypt_file(target, &crypto_key);
        
            // 1.2. Upload file to ciphertext to IPFS and get CID (content identifier)
            let cid = ipfs::upload_to_ipfs(ciphertext);

            // 1.3. Register the key after uploading the cipher text and getting cid
            client::register_key(cid, crypto_key, DEFAULT_TTL_SECONDS);
            return cid

        },
        "retrieve" => {
            println!("Starting download flow for CID: {}", target);
            // 2. retrieve file

            // 2.1. let ciphertext = ipfs::download(target);
            // Download file with corresponding CID
            let ciphertext = ipfs::download(target);

            // 2.2. Contact go server with the client to get the key
            let crypto_key = client::get_key(target);

            // 2.3. Decrypt the file wih the key
            let plaintext = crypto::decrypt_file(&ciphertext, &crypto_key);
            return plaintext
        },
        _ => {
            // default case
            println!("Unknown command. Please use 'publish' or 'retrieve' in order to either publish or retrieve the file/s.");
        }
    }
}