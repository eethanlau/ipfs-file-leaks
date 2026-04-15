fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=../proto/key_service.proto");

    // Use tonic_build to automatically generate the Rust gRPC code
    tonic_build::compile_protos("../proto/key_service.proto")?;
    
    Ok(())
}
