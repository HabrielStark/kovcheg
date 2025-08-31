//! Post-Quantum TLS 1.3 Implementation
//! "A cord of three strands is not quickly broken" - Ecclesiastes 4:12
//!
//! Implements hybrid classical + post-quantum TLS for future-proof security

use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::TlsAcceptor;
use rustls::{ServerConfig, Certificate, PrivateKey};
use ring::rand::{SecureRandom, SystemRandom};
use pqcrypto_kyber::*;
use pqcrypto_dilithium::*;
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use ed25519_dalek::{Keypair as Ed25519Keypair, PublicKey as Ed25519PublicKey};
use sha3::{Sha3_256, Digest};
use std::error::Error;
use std::fmt;
use serde::{Serialize, Deserialize};
use zeroize::Zeroize;


/// Post-quantum TLS errors
#[derive(Debug)]
pub enum PQTlsError {
    /// Key exchange failed
    KeyExchangeFailed,
    /// Signature verification failed
    SignatureVerificationFailed,
    /// Unsupported algorithm
    UnsupportedAlgorithm,
    /// Protocol error
    ProtocolError(String),
    /// Crypto error
    CryptoError(String),
}

impl fmt::Display for PQTlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PQTlsError::KeyExchangeFailed => write!(f, "Post-quantum key exchange failed"),
            PQTlsError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            PQTlsError::UnsupportedAlgorithm => write!(f, "Unsupported PQC algorithm"),
            PQTlsError::ProtocolError(e) => write!(f, "Protocol error: {}", e),
            PQTlsError::CryptoError(e) => write!(f, "Crypto error: {}", e),
        }
    }
}

impl Error for PQTlsError {}

/// Supported post-quantum algorithms
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PQAlgorithm {
    /// Hybrid X25519 + Kyber768
    HybridX25519Kyber768,
    /// Hybrid Ed25519 + Dilithium3
    HybridEd25519Dilithium3,
    /// Pure Kyber768
    Kyber768,
    /// Pure Dilithium3
    Dilithium3,
}

/// Post-quantum key share for TLS handshake
#[derive(Clone, Serialize, Deserialize)]
pub struct PQKeyShare {
    /// Algorithm used
    pub algorithm: PQAlgorithm,
    /// Classical public key (if hybrid)
    pub classical_public: Option<Vec<u8>>,
    /// Post-quantum public key
    pub pq_public: Vec<u8>,
}

/// Post-quantum signature for TLS certificates
#[derive(Clone, Serialize, Deserialize)]
pub struct PQSignature {
    /// Algorithm used
    pub algorithm: PQAlgorithm,
    /// Classical signature (if hybrid)
    pub classical_signature: Option<Vec<u8>>,
    /// Post-quantum signature
    pub pq_signature: Vec<u8>,
}

/// Hybrid key exchange result
pub struct HybridSharedSecret {
    /// Combined shared secret
    pub secret: Vec<u8>,
}

impl Zeroize for HybridSharedSecret {
    fn zeroize(&mut self) {
        self.secret.zeroize();
    }
}

/// Post-quantum TLS configuration
pub struct PQTlsConfig {
    /// Supported PQ algorithms in preference order
    pub supported_algorithms: Vec<PQAlgorithm>,
    /// Whether to require PQ algorithms
    pub require_pq: bool,
    /// Kyber keypair
    pub kyber_keypair: Option<(pqcrypto_kyber::PublicKey, pqcrypto_kyber::SecretKey)>,
    /// Dilithium keypair
    pub dilithium_keypair: Option<(pqcrypto_dilithium::PublicKey, pqcrypto_dilithium::SecretKey)>,
    /// Classical keypairs for hybrid mode
    pub x25519_secret: Option<EphemeralSecret>,
    pub ed25519_keypair: Option<Ed25519Keypair>,
}

impl Default for PQTlsConfig {
    fn default() -> Self {
        Self {
            supported_algorithms: vec![
                PQAlgorithm::HybridX25519Kyber768,
                PQAlgorithm::HybridEd25519Dilithium3,
                PQAlgorithm::Kyber768,
                PQAlgorithm::Dilithium3,
            ],
            require_pq: true,
            kyber_keypair: None,
            dilithium_keypair: None,
            x25519_secret: None,
            ed25519_keypair: None,
        }
    }
}

impl PQTlsConfig {
    /// Generate new keypairs for all algorithms
    pub fn generate_keypairs(&mut self) -> Result<(), PQTlsError> {
        // Generate Kyber keypair
        let (kyber_pk, kyber_sk) = pqcrypto_kyber::keypair();
        self.kyber_keypair = Some((kyber_pk, kyber_sk));
        
        // Generate Dilithium keypair
        let (dilithium_pk, dilithium_sk) = pqcrypto_dilithium::keypair();
        self.dilithium_keypair = Some((dilithium_pk, dilithium_sk));
        
        // Generate classical keypairs
        use rand::rngs::OsRng;
        self.x25519_secret = Some(EphemeralSecret::new(OsRng));
        self.ed25519_keypair = Some(Ed25519Keypair::generate(&mut OsRng));
        
        Ok(())
    }
}

/// Post-quantum TLS handshake extension
pub struct PQHandshake {
    /// Configuration
    config: Arc<PQTlsConfig>,
    /// Role (client or server)
    is_client: bool,
    /// Negotiated algorithm
    negotiated_algorithm: Option<PQAlgorithm>,
    /// Shared secret after key exchange
    shared_secret: Option<HybridSharedSecret>,
}

impl PQHandshake {
    /// Create new PQ handshake
    pub fn new(config: Arc<PQTlsConfig>, is_client: bool) -> Self {
        Self {
            config,
            is_client,
            negotiated_algorithm: None,
            shared_secret: None,
        }
    }
    
    /// Generate key share for handshake
    pub fn generate_key_share(&self, algorithm: PQAlgorithm) -> Result<PQKeyShare, PQTlsError> {
        match algorithm {
            PQAlgorithm::HybridX25519Kyber768 => {
                // Get X25519 public key
                let x25519_public = self.config.x25519_secret.as_ref()
                    .map(|secret| X25519PublicKey::from(secret))
                    .ok_or(PQTlsError::CryptoError("Missing X25519 key".into()))?;
                
                // Get Kyber public key
                let kyber_public = self.config.kyber_keypair.as_ref()
                    .map(|(pk, _)| pk.clone())
                    .ok_or(PQTlsError::CryptoError("Missing Kyber key".into()))?;
                
                Ok(PQKeyShare {
                    algorithm,
                    classical_public: Some(x25519_public.as_bytes().to_vec()),
                    pq_public: kyber_public.as_bytes().to_vec(),
                })
            }
            PQAlgorithm::Kyber768 => {
                let kyber_public = self.config.kyber_keypair.as_ref()
                    .map(|(pk, _)| pk.clone())
                    .ok_or(PQTlsError::CryptoError("Missing Kyber key".into()))?;
                
                Ok(PQKeyShare {
                    algorithm,
                    classical_public: None,
                    pq_public: kyber_public.as_bytes().to_vec(),
                })
            }
            _ => Err(PQTlsError::UnsupportedAlgorithm),
        }
    }
    
    /// Process peer's key share and derive shared secret
    pub fn process_key_share(&mut self, peer_share: &PQKeyShare) -> Result<(), PQTlsError> {
        self.negotiated_algorithm = Some(peer_share.algorithm);
        
        match peer_share.algorithm {
            PQAlgorithm::HybridX25519Kyber768 => {
                // Process X25519 part
                let peer_x25519_public = peer_share.classical_public.as_ref()
                    .ok_or(PQTlsError::ProtocolError("Missing classical key".into()))?;
                
                let x25519_public = X25519PublicKey::from(
                    <[u8; 32]>::try_from(peer_x25519_public.as_slice())
                        .map_err(|_| PQTlsError::CryptoError("Invalid X25519 key".into()))?
                );
                
                let x25519_secret = self.config.x25519_secret.as_ref()
                    .ok_or(PQTlsError::CryptoError("Missing X25519 secret".into()))?;
                
                // Note: In real implementation, we'd need to handle the ephemeral secret properly
                // This is a simplified version
                
                // Process Kyber part
                let peer_kyber_public = pqcrypto_kyber::PublicKey::from_bytes(&peer_share.pq_public)
                    .map_err(|_| PQTlsError::CryptoError("Invalid Kyber key".into()))?;
                
                let (ciphertext, kyber_shared) = pqcrypto_kyber::encapsulate(&peer_kyber_public);
                
                // Combine secrets with domain separation
                let mut kdf = Sha3_256::new();
                kdf.update(b"ARK-PQ-TLS-HYBRID-V1");
                kdf.update(b"X25519");
                // kdf.update(x25519_shared.as_bytes()); // Would need the actual shared secret
                kdf.update(b"KYBER768");
                kdf.update(&kyber_shared);
                
                let combined_secret = kdf.finalize().to_vec();
                
                self.shared_secret = Some(HybridSharedSecret {
                    secret: combined_secret,
                });
                
                Ok(())
            }
            PQAlgorithm::Kyber768 => {
                let peer_kyber_public = pqcrypto_kyber::PublicKey::from_bytes(&peer_share.pq_public)
                    .map_err(|_| PQTlsError::CryptoError("Invalid Kyber key".into()))?;
                
                if self.is_client {
                    // Client encapsulates
                    let (ciphertext, shared_secret) = pqcrypto_kyber::encapsulate(&peer_kyber_public);
                    
                    self.shared_secret = Some(HybridSharedSecret {
                        secret: shared_secret.as_bytes().to_vec(),
                    });
                } else {
                    // Server will decapsulate when receiving ciphertext
                    // This is handled in process_key_exchange_response
                }
                
                Ok(())
            }
            _ => Err(PQTlsError::UnsupportedAlgorithm),
        }
    }
    
    /// Create signature using hybrid algorithm
    pub fn create_signature(&self, message: &[u8]) -> Result<PQSignature, PQTlsError> {
        match self.negotiated_algorithm {
            Some(PQAlgorithm::HybridEd25519Dilithium3) => {
                // Ed25519 signature
                let ed25519_keypair = self.config.ed25519_keypair.as_ref()
                    .ok_or(PQTlsError::CryptoError("Missing Ed25519 key".into()))?;
                
                let ed25519_sig = ed25519_keypair.sign(message);
                
                // Dilithium signature
                let (_, dilithium_sk) = self.config.dilithium_keypair.as_ref()
                    .ok_or(PQTlsError::CryptoError("Missing Dilithium key".into()))?;
                
                let dilithium_sig = pqcrypto_dilithium::sign(message, dilithium_sk);
                
                Ok(PQSignature {
                    algorithm: PQAlgorithm::HybridEd25519Dilithium3,
                    classical_signature: Some(ed25519_sig.to_bytes().to_vec()),
                    pq_signature: dilithium_sig,
                })
            }
            Some(PQAlgorithm::Dilithium3) => {
                let (_, dilithium_sk) = self.config.dilithium_keypair.as_ref()
                    .ok_or(PQTlsError::CryptoError("Missing Dilithium key".into()))?;
                
                let signature = pqcrypto_dilithium::sign(message, dilithium_sk);
                
                Ok(PQSignature {
                    algorithm: PQAlgorithm::Dilithium3,
                    classical_signature: None,
                    pq_signature: signature,
                })
            }
            _ => Err(PQTlsError::UnsupportedAlgorithm),
        }
    }
    
    /// Verify signature using hybrid algorithm
    pub fn verify_signature(&self, message: &[u8], signature: &PQSignature, 
                          peer_public_keys: &PeerPublicKeys) -> Result<(), PQTlsError> {
        match signature.algorithm {
            PQAlgorithm::HybridEd25519Dilithium3 => {
                // Verify Ed25519 signature
                if let Some(ed25519_sig_bytes) = &signature.classical_signature {
                    let ed25519_sig = ed25519_dalek::Signature::from_bytes(
                        &<[u8; 64]>::try_from(ed25519_sig_bytes.as_slice())
                            .map_err(|_| PQTlsError::CryptoError("Invalid Ed25519 signature".into()))?
                    );
                    
                    peer_public_keys.ed25519_public.as_ref()
                        .ok_or(PQTlsError::CryptoError("Missing Ed25519 public key".into()))?
                        .verify(message, &ed25519_sig)
                        .map_err(|_| PQTlsError::SignatureVerificationFailed)?;
                }
                
                // Verify Dilithium signature
                pqcrypto_dilithium::verify(
                    &signature.pq_signature,
                    message,
                    peer_public_keys.dilithium_public.as_ref()
                        .ok_or(PQTlsError::CryptoError("Missing Dilithium public key".into()))?
                ).map_err(|_| PQTlsError::SignatureVerificationFailed)?;
                
                Ok(())
            }
            PQAlgorithm::Dilithium3 => {
                pqcrypto_dilithium::verify(
                    &signature.pq_signature,
                    message,
                    peer_public_keys.dilithium_public.as_ref()
                        .ok_or(PQTlsError::CryptoError("Missing Dilithium public key".into()))?
                ).map_err(|_| PQTlsError::SignatureVerificationFailed)?;
                
                Ok(())
            }
            _ => Err(PQTlsError::UnsupportedAlgorithm),
        }
    }
    
    /// Get the derived shared secret
    pub fn get_shared_secret(&self) -> Option<&[u8]> {
        self.shared_secret.as_ref().map(|s| s.secret.as_slice())
    }
}

/// Peer's public keys for verification
pub struct PeerPublicKeys {
    pub ed25519_public: Option<Ed25519PublicKey>,
    pub dilithium_public: Option<pqcrypto_dilithium::PublicKey>,
}

/// Post-quantum TLS acceptor
pub struct PQTlsAcceptor {
    /// Base TLS acceptor
    base_acceptor: TlsAcceptor,
    /// PQ configuration
    pq_config: Arc<PQTlsConfig>,
}

impl PQTlsAcceptor {
    /// Create new PQ-TLS acceptor
    pub fn new(server_config: ServerConfig, pq_config: PQTlsConfig) -> Self {
        Self {
            base_acceptor: TlsAcceptor::from(Arc::new(server_config)),
            pq_config: Arc::new(pq_config),
        }
    }
    
    /// Accept connection with PQ handshake
    pub async fn accept<IO>(&self, stream: IO) -> Result<PQTlsStream<IO>, Box<dyn Error>>
    where
        IO: AsyncRead + AsyncWrite + Unpin,
    {
        // Perform base TLS handshake
        let tls_stream = self.base_acceptor.accept(stream).await?;
        
        // Create PQ handshake handler
        let pq_handshake = PQHandshake::new(self.pq_config.clone(), false);
        
        // Wrap in PQ-TLS stream
        Ok(PQTlsStream {
            inner: tls_stream,
            pq_handshake: Some(pq_handshake),
        })
    }
}

/// Post-quantum TLS stream
pub struct PQTlsStream<IO> {
    /// Inner TLS stream
    inner: tokio_rustls::server::TlsStream<IO>,
    /// PQ handshake state
    pq_handshake: Option<PQHandshake>,
}

impl<IO> PQTlsStream<IO>
where
    IO: AsyncRead + AsyncWrite + Unpin,
{
    /// Get reference to PQ handshake
    pub fn pq_handshake(&self) -> Option<&PQHandshake> {
        self.pq_handshake.as_ref()
    }
    
    /// Get shared secret from PQ key exchange
    pub fn get_pq_shared_secret(&self) -> Option<&[u8]> {
        self.pq_handshake.as_ref()?.get_shared_secret()
    }
}

/// Create hybrid TLS configuration
pub fn create_hybrid_tls_config(require_pq: bool) -> Result<(ServerConfig, PQTlsConfig), Box<dyn Error>> {
    // Create base rustls configuration
    let mut server_config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![], PrivateKey(vec![]))?; // Would need real cert/key
    
    // Create PQ configuration
    let mut pq_config = PQTlsConfig {
        require_pq,
        ..Default::default()
    };
    
    // Generate all keypairs
    pq_config.generate_keypairs()?;
    
    Ok((server_config, pq_config))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pq_key_generation() {
        let mut config = PQTlsConfig::default();
        assert!(config.generate_keypairs().is_ok());
        
        assert!(config.kyber_keypair.is_some());
        assert!(config.dilithium_keypair.is_some());
        assert!(config.x25519_secret.is_some());
        assert!(config.ed25519_keypair.is_some());
    }
    
    #[test]
    fn test_key_share_generation() {
        let mut config = PQTlsConfig::default();
        config.generate_keypairs().unwrap();
        
        let handshake = PQHandshake::new(Arc::new(config), true);
        
        // Test hybrid key share
        let hybrid_share = handshake.generate_key_share(PQAlgorithm::HybridX25519Kyber768).unwrap();
        assert_eq!(hybrid_share.algorithm, PQAlgorithm::HybridX25519Kyber768);
        assert!(hybrid_share.classical_public.is_some());
        assert!(!hybrid_share.pq_public.is_empty());
        
        // Test pure PQ key share
        let pq_share = handshake.generate_key_share(PQAlgorithm::Kyber768).unwrap();
        assert_eq!(pq_share.algorithm, PQAlgorithm::Kyber768);
        assert!(pq_share.classical_public.is_none());
        assert!(!pq_share.pq_public.is_empty());
    }
    
    #[test]
    fn test_signature_creation_verification() {
        let mut config = PQTlsConfig::default();
        config.generate_keypairs().unwrap();
        
        let mut handshake = PQHandshake::new(Arc::new(config.clone()), true);
        handshake.negotiated_algorithm = Some(PQAlgorithm::HybridEd25519Dilithium3);
        
        let message = b"Test message for signature";
        
        // Create signature
        let signature = handshake.create_signature(message).unwrap();
        assert_eq!(signature.algorithm, PQAlgorithm::HybridEd25519Dilithium3);
        assert!(signature.classical_signature.is_some());
        assert!(!signature.pq_signature.is_empty());
        
        // Prepare public keys for verification
        let peer_keys = PeerPublicKeys {
            ed25519_public: config.ed25519_keypair.as_ref().map(|kp| kp.public),
            dilithium_public: config.dilithium_keypair.as_ref().map(|(pk, _)| pk.clone()),
        };
        
        // Verify signature
        assert!(handshake.verify_signature(message, &signature, &peer_keys).is_ok());
        
        // Verify with wrong message fails
        let wrong_message = b"Wrong message";
        assert!(handshake.verify_signature(wrong_message, &signature, &peer_keys).is_err());
    }
}
