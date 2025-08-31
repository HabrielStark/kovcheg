//! Network Sentinel - Post-Quantum Secure Communications Library
//! "The Lord watches over all who love him" - Psalm 145:20

pub mod pqc_tls;

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, error};
use thiserror::Error;

pub use pqc_tls::{PQTlsConfig, PQTlsAcceptor, PQTlsStream, PQAlgorithm};

/// Network Sentinel errors
#[derive(Error, Debug)]
pub enum SentinelError {
    #[error("Post-quantum TLS error: {0}")]
    PQTlsError(#[from] pqc_tls::PQTlsError),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
}

/// Network Sentinel configuration
#[derive(Clone)]
pub struct SentinelConfig {
    /// Listen address
    pub bind_addr: SocketAddr,
    /// PQ-TLS configuration
    pub pq_tls_config: Arc<PQTlsConfig>,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Enable quantum-resistant mode
    pub quantum_resistant: bool,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        let mut pq_config = PQTlsConfig::default();
        pq_config.require_pq = true;
        
        Self {
            bind_addr: "127.0.0.1:8443".parse().unwrap(),
            pq_tls_config: Arc::new(pq_config),
            max_connections: 1000,
            connection_timeout: 30,
            quantum_resistant: true,
        }
    }
}

/// Network Sentinel server
pub struct NetworkSentinel {
    config: SentinelConfig,
    listener: Option<TcpListener>,
}

impl NetworkSentinel {
    /// Create new Network Sentinel
    pub fn new(config: SentinelConfig) -> Self {
        Self {
            config,
            listener: None,
        }
    }
    
    /// Initialize and bind to address
    pub async fn initialize(&mut self) -> Result<(), SentinelError> {
        info!("Initializing Network Sentinel on {}", self.config.bind_addr);
        
        // Generate PQ keypairs if needed
        if self.config.quantum_resistant {
            Arc::get_mut(&mut self.config.pq_tls_config)
                .ok_or_else(|| SentinelError::ConfigError("Cannot modify shared config".into()))?
                .generate_keypairs()
                .map_err(|e| SentinelError::PQTlsError(e))?;
            
            info!("Post-quantum keypairs generated successfully");
        }
        
        // Bind to address
        self.listener = Some(TcpListener::bind(self.config.bind_addr).await?);
        
        info!("Network Sentinel initialized and listening");
        Ok(())
    }
    
    /// Run the server
    pub async fn run(&mut self) -> Result<(), SentinelError> {
        let listener = self.listener.as_ref()
            .ok_or_else(|| SentinelError::ConfigError("Server not initialized".into()))?;
        
        info!("Network Sentinel running in {} mode", 
              if self.config.quantum_resistant { "quantum-resistant" } else { "classical" });
        
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);
                    
                    let config = self.config.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, config).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle individual connection
async fn handle_connection(
    mut stream: TcpStream,
    config: SentinelConfig,
) -> Result<(), SentinelError> {
    // Set connection timeout
    let timeout = tokio::time::Duration::from_secs(config.connection_timeout);
    
    // In a real implementation, we would perform the PQ-TLS handshake here
    // For now, we'll demonstrate the protocol flow
    
    if config.quantum_resistant {
        info!("Initiating post-quantum handshake");
        
        // Send supported algorithms
        let supported_algos = vec![
            PQAlgorithm::HybridX25519Kyber768,
            PQAlgorithm::HybridEd25519Dilithium3,
        ];
        
        let algo_bytes = bincode::serialize(&supported_algos)
            .map_err(|e| SentinelError::ProtocolError(e.to_string()))?;
        
        stream.write_u32(algo_bytes.len() as u32).await?;
        stream.write_all(&algo_bytes).await?;
        
        // Read client's choice
        let choice_len = stream.read_u32().await? as usize;
        let mut choice_buf = vec![0u8; choice_len];
        stream.read_exact(&mut choice_buf).await?;
        
        let chosen_algo: PQAlgorithm = bincode::deserialize(&choice_buf)
            .map_err(|e| SentinelError::ProtocolError(e.to_string()))?;
        
        info!("Client chose algorithm: {:?}", chosen_algo);
        
        // Continue with PQ-TLS handshake...
        // This would integrate with the pqc_tls module
    }
    
    // Echo server for demonstration
    let mut buf = [0; 1024];
    loop {
        match tokio::time::timeout(timeout, stream.read(&mut buf)).await {
            Ok(Ok(0)) => {
                info!("Connection closed");
                break;
            }
            Ok(Ok(n)) => {
                // Echo back
                stream.write_all(&buf[..n]).await?;
            }
            Ok(Err(e)) => {
                error!("Read error: {}", e);
                break;
            }
            Err(_) => {
                warn!("Connection timeout");
                break;
            }
        }
    }
    
    Ok(())
}

/// Client connection with PQ-TLS
pub struct SentinelClient {
    config: SentinelConfig,
}

impl SentinelClient {
    /// Create new client
    pub fn new(quantum_resistant: bool) -> Self {
        let mut pq_config = PQTlsConfig::default();
        pq_config.require_pq = quantum_resistant;
        
        let config = SentinelConfig {
            quantum_resistant,
            pq_tls_config: Arc::new(pq_config),
            ..Default::default()
        };
        
        Self { config }
    }
    
    /// Connect to server
    pub async fn connect(&mut self, addr: SocketAddr) -> Result<TcpStream, SentinelError> {
        info!("Connecting to {} with {} security", addr,
              if self.config.quantum_resistant { "post-quantum" } else { "classical" });
        
        // Generate client keypairs
        if self.config.quantum_resistant {
            Arc::get_mut(&mut self.config.pq_tls_config)
                .ok_or_else(|| SentinelError::ConfigError("Cannot modify shared config".into()))?
                .generate_keypairs()
                .map_err(|e| SentinelError::PQTlsError(e))?;
        }
        
        let mut stream = TcpStream::connect(addr).await?;
        
        if self.config.quantum_resistant {
            // Read server's supported algorithms
            let algo_len = stream.read_u32().await? as usize;
            let mut algo_buf = vec![0u8; algo_len];
            stream.read_exact(&mut algo_buf).await?;
            
            let supported_algos: Vec<PQAlgorithm> = bincode::deserialize(&algo_buf)
                .map_err(|e| SentinelError::ProtocolError(e.to_string()))?;
            
            info!("Server supports: {:?}", supported_algos);
            
            // Choose first supported algorithm
            let chosen = supported_algos.first()
                .ok_or_else(|| SentinelError::ProtocolError("No supported algorithms".into()))?;
            
            let choice_bytes = bincode::serialize(chosen)
                .map_err(|e| SentinelError::ProtocolError(e.to_string()))?;
            
            stream.write_u32(choice_bytes.len() as u32).await?;
            stream.write_all(&choice_bytes).await?;
            
            info!("Chose algorithm: {:?}", chosen);
            
            // Continue with PQ-TLS handshake...
        }
        
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sentinel_initialization() {
        let config = SentinelConfig::default();
        let mut sentinel = NetworkSentinel::new(config);
        
        // Should initialize successfully
        assert!(sentinel.initialize().await.is_ok());
        assert!(sentinel.listener.is_some());
    }
    
    #[tokio::test]
    async fn test_client_creation() {
        let mut client = SentinelClient::new(true);
        
        // Client should be created with quantum-resistant mode
        assert!(client.config.quantum_resistant);
        assert!(client.config.pq_tls_config.require_pq);
    }
}
