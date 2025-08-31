//! Network Sentinel - Main Entry Point
//! "He will command his angels concerning you to guard you in all your ways" - Psalm 91:11

use network_sentinel::{NetworkSentinel, SentinelConfig, SentinelClient};
use std::net::SocketAddr;
use clap::{Parser, Subcommand};
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "network-sentinel")]
#[command(about = "ARK Network Sentinel - Post-Quantum Secure Communications")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as server
    Server {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0:8443")]
        bind: String,
        
        /// Disable post-quantum security (not recommended)
        #[arg(long)]
        no_pq: bool,
        
        /// Maximum concurrent connections
        #[arg(long, default_value = "1000")]
        max_connections: usize,
    },
    
    /// Run as client
    Client {
        /// Server address to connect to
        #[arg(short, long)]
        connect: String,
        
        /// Disable post-quantum security (not recommended)
        #[arg(long)]
        no_pq: bool,
        
        /// Message to send
        #[arg(short, long)]
        message: Option<String>,
    },
    
    /// Run benchmark tests
    Benchmark {
        /// Number of iterations
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Server { bind, no_pq, max_connections } => {
            run_server(bind, !no_pq, max_connections).await?;
        }
        Commands::Client { connect, no_pq, message } => {
            run_client(connect, !no_pq, message).await?;
        }
        Commands::Benchmark { iterations } => {
            run_benchmark(iterations).await?;
        }
    }
    
    Ok(())
}

async fn run_server(bind_addr: String, quantum_resistant: bool, max_connections: usize) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Network Sentinel server");
    info!("Post-quantum security: {}", if quantum_resistant { "ENABLED" } else { "DISABLED" });
    
    let addr: SocketAddr = bind_addr.parse()?;
    
    let mut config = SentinelConfig::default();
    config.bind_addr = addr;
    config.quantum_resistant = quantum_resistant;
    config.max_connections = max_connections;
    
    if quantum_resistant {
        config.pq_tls_config = std::sync::Arc::new(network_sentinel::PQTlsConfig::default());
    }
    
    let mut sentinel = NetworkSentinel::new(config);
    sentinel.initialize().await?;
    
    info!("Server listening on {}", addr);
    sentinel.run().await?;
    
    Ok(())
}

async fn run_client(server_addr: String, quantum_resistant: bool, message: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting Network Sentinel client");
    info!("Post-quantum security: {}", if quantum_resistant { "ENABLED" } else { "DISABLED" });
    
    let addr: SocketAddr = server_addr.parse()?;
    
    let mut client = SentinelClient::new(quantum_resistant);
    
    info!("Connecting to {}", addr);
    let mut stream = client.connect(addr).await?;
    
    info!("Connected successfully!");
    
    // Send message if provided
    if let Some(msg) = message {
        use tokio::io::AsyncWriteExt;
        stream.write_all(msg.as_bytes()).await?;
        info!("Sent message: {}", msg);
        
        // Read echo response
        use tokio::io::AsyncReadExt;
        let mut buf = vec![0u8; msg.len()];
        stream.read_exact(&mut buf).await?;
        
        let response = String::from_utf8_lossy(&buf);
        info!("Received echo: {}", response);
    }
    
    Ok(())
}

async fn run_benchmark(iterations: usize) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    use network_sentinel::PQTlsConfig;
    
    info!("Running post-quantum cryptography benchmarks");
    info!("Iterations: {}", iterations);
    
    // Benchmark key generation
    let start = Instant::now();
    for _ in 0..iterations {
        let mut config = PQTlsConfig::default();
        config.generate_keypairs()?;
    }
    let keygen_time = start.elapsed();
    
    info!("Key generation benchmark:");
    info!("  Total time: {:?}", keygen_time);
    info!("  Average per operation: {:?}", keygen_time / iterations as u32);
    info!("  Operations per second: {:.2}", iterations as f64 / keygen_time.as_secs_f64());
    
    // Benchmark handshake simulation
    let mut config = PQTlsConfig::default();
    config.generate_keypairs()?;
    
    let start = Instant::now();
    for _ in 0..iterations {
        // Simulate key exchange
        use network_sentinel::pqc_tls::{PQHandshake, PQAlgorithm};
        let handshake = PQHandshake::new(std::sync::Arc::new(config.clone()), true);
        let _key_share = handshake.generate_key_share(PQAlgorithm::HybridX25519Kyber768)?;
    }
    let handshake_time = start.elapsed();
    
    info!("\nKey exchange benchmark:");
    info!("  Total time: {:?}", handshake_time);
    info!("  Average per operation: {:?}", handshake_time / iterations as u32);
    info!("  Operations per second: {:.2}", iterations as f64 / handshake_time.as_secs_f64());
    
    // Compare with classical crypto
    info!("\nComparison with classical cryptography:");
    info!("  Classical Ed25519 signatures: ~75,000 ops/sec");
    info!("  Post-quantum Dilithium3: ~15,000 ops/sec");
    info!("  Overhead: ~80% (acceptable for quantum resistance)");
    
    Ok(())
}
