fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile protos if proto files exist and protoc is available
    if std::path::Path::new("proto/database.proto").exists() {
        match tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .out_dir("src/network")
            .compile_protos(&["proto/database.proto"], &["proto/"])
        {
            Ok(_) => println!("Successfully compiled protobuf files"),
            Err(e) => {
                println!("cargo:warning=Failed to compile protos: {}. Continuing without proto compilation.", e);
                println!("cargo:warning=Install protobuf-compiler package or set PROTOC environment variable");
            }
        }
    } else {
        println!("cargo:warning=No proto files found, skipping protobuf compilation");
    }

    Ok(())
}
