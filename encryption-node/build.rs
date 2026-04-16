fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use the bundled protoc — no system install needed!
    let protoc_path = protoc_bin_vendored::protoc_bin_path().unwrap();
    std::env::set_var("PROTOC", protoc_path);

    println!("cargo:rerun-if-changed=../proto/key_service.proto");

    // Use tonic_build to automatically generate the Rust gRPC code
    tonic_build::compile_protos("../proto/key_service.proto")?;
    
    Ok(())
}
