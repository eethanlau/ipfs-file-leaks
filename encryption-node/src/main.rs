use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;

use encryption_node::config::Config;
use encryption_node::pipeline::Pipeline;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: cargo run <publish|retrieve> <file_path|cid>");
        return;
    }

    let command = &args[1];
    let target = &args[2];

    let pipeline = Pipeline::new(Config::default_localhost());

    match command.as_str() {
        "publish" => {
            println!("Starting upload flow for file: {}", target);
            let plaintext = read_file(target.to_string()).expect("Failed to read file contents");
            let outcome = pipeline
                .publish(&plaintext, pipeline.default_ttl())
                .await
                .expect("publish failed");
            println!("File published, CID: {}", outcome.cid);
        }
        "retrieve" => {
            println!("Starting download flow for CID: {}", target);
            let plaintext = pipeline.retrieve(target).await.expect("retrieve failed");
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
