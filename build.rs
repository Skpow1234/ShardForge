fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure protobuf compilation
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/network")
        .compile(&["proto/database.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos: {}", e));

    Ok(())
}
