//! Network Sentinel CLI.
//!
//! The PQ-TLS data plane is a placeholder today (see `pqc_tls.rs`). The CLI
//! exists to keep the workspace coherent and to document the future
//! `server` / `client` / `benchmark` subcommand surface.

use clap::{Parser, Subcommand};
use network_sentinel::{NetworkSentinel, PQTlsError, SentinelConfig};

#[derive(Parser)]
#[command(name = "network-sentinel")]
#[command(about = "ARK Network Sentinel - Post-Quantum Secure Communications")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as a server (placeholder).
    Server {
        /// Bind address.
        #[arg(short, long, default_value = "0.0.0.0:8443")]
        bind: String,
    },
    /// Run as a client (placeholder).
    Client {
        /// Server address to connect to.
        #[arg(short, long)]
        connect: String,
    },
    /// Run cryptography benchmarks (placeholder).
    Benchmark {
        /// Number of iterations.
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let _ = NetworkSentinel::new(SentinelConfig::default());

    match cli.command {
        Commands::Server { bind } => {
            println!("Network Sentinel server placeholder bound to {bind}.");
        }
        Commands::Client { connect } => {
            println!("Network Sentinel client placeholder targeting {connect}.");
        }
        Commands::Benchmark { iterations } => {
            println!("Network Sentinel benchmark placeholder ({iterations} iterations).");
        }
    }

    // The real handshake will surface NotImplemented until it lands.
    let _: PQTlsError = PQTlsError::NotImplemented;
    Ok(())
}
