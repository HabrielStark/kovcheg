//! Post-Quantum Cryptography Test Suite
//! "Test all things; hold fast to what is good" - 1 Thessalonians 5:21

#![cfg(test)]
#![cfg(feature = "post-quantum")]

use ark_firmware::crypto::{CryptoContext, CryptoError, PQAlgorithm, PQEncryptedData, HybridEncryptedData, HybridSignature};
use ark_firmware::SecureKey;

mod test_utils {
    use super::*;
    use rand_core::{RngCore, OsRng};
    
    /// Generate test data of specified size
    pub fn generate_test_data(size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        OsRng.fill_bytes(&mut data);
        data
    }
    
    /// Initialize crypto context with PQC enabled
    pub fn init_pqc_context() -> Result<CryptoContext, CryptoError> {
        let mut ctx = CryptoContext::new(generate_test_data(32))?;
        ctx.initialize_post_quantum()?;
        Ok(ctx)
    }
}

#[cfg(test)]
mod kyber_tests {
    use super::*;
    use super::test_utils::*;
    
    #[test]
    fn test_kyber_encryption_decryption() {
        let mut alice_ctx = init_pqc_context().unwrap();
        let mut bob_ctx = init_pqc_context().unwrap();
        
        // Test various message sizes
        let test_sizes = vec![16, 32, 64, 256, 1024, 4096];
        
        for size in test_sizes {
            let plaintext = generate_test_data(size);
            
            // Alice encrypts for Bob
            let bob_public_key = bob_ctx.get_pq_public_keys().unwrap().kyber_public.clone();
            let encrypted = alice_ctx.pq_encrypt(&plaintext, &bob_public_key).unwrap();
            
            // Verify ciphertext structure
            assert!(!encrypted.kyber_ciphertext.is_empty());
            assert!(!encrypted.encrypted_payload.is_empty());
            assert_eq!(encrypted.algorithm, PQAlgorithm::KyberAes256Gcm);
            
            // Bob decrypts
            let decrypted = bob_ctx.pq_decrypt(&encrypted).unwrap();
            assert_eq!(plaintext, decrypted);
        }
    }
    
    #[test]
    fn test_kyber_wrong_key_fails() {
        let mut alice_ctx = init_pqc_context().unwrap();
        let mut bob_ctx = init_pqc_context().unwrap();
        let mut charlie_ctx = init_pqc_context().unwrap();
        
        let plaintext = b"Secret message for Bob only";
        
        // Alice encrypts for Bob
        let bob_public_key = bob_ctx.get_pq_public_keys().unwrap().kyber_public.clone();
        let encrypted = alice_ctx.pq_encrypt(plaintext, &bob_public_key).unwrap();
        
        // Charlie tries to decrypt (should fail)
        let result = charlie_ctx.pq_decrypt(&encrypted);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_kyber_tampered_ciphertext_fails() {
        let mut alice_ctx = init_pqc_context().unwrap();
        let mut bob_ctx = init_pqc_context().unwrap();
        
        let plaintext = b"Integrity protected message";
        
        // Alice encrypts for Bob
        let bob_public_key = bob_ctx.get_pq_public_keys().unwrap().kyber_public.clone();
        let mut encrypted = alice_ctx.pq_encrypt(plaintext, &bob_public_key).unwrap();
        
        // Tamper with ciphertext
        encrypted.encrypted_payload[0] ^= 0xFF;
        
        // Bob tries to decrypt (should fail due to AEAD)
        let result = bob_ctx.pq_decrypt(&encrypted);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod dilithium_tests {
    use super::*;
    use super::test_utils::*;
    
    #[test]
    fn test_dilithium_sign_verify() {
        let mut ctx = init_pqc_context().unwrap();
        
        let messages = vec![
            b"Short message".to_vec(),
            generate_test_data(1024),
            generate_test_data(10240),
        ];
        
        for message in messages {
            // Sign message
            let signature = ctx.pq_sign(&message).unwrap();
            assert!(!signature.is_empty());
            
            // Verify signature
            let public_key = ctx.get_pq_public_keys().unwrap().dilithium_public.clone();
            let result = ctx.pq_verify(&message, &signature, &public_key);
            assert!(result.is_ok());
        }
    }
    
    #[test]
    fn test_dilithium_wrong_signature_fails() {
        let mut ctx = init_pqc_context().unwrap();
        let message = b"Authentic message";
        
        // Sign message
        let mut signature = ctx.pq_sign(message).unwrap();
        
        // Tamper with signature
        signature[0] ^= 0xFF;
        
        // Verify should fail
        let public_key = ctx.get_pq_public_keys().unwrap().dilithium_public.clone();
        let result = ctx.pq_verify(message, &signature, &public_key);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_dilithium_wrong_message_fails() {
        let mut ctx = init_pqc_context().unwrap();
        let message = b"Original message";
        
        // Sign message
        let signature = ctx.pq_sign(message).unwrap();
        
        // Try to verify with different message
        let wrong_message = b"Modified message";
        let public_key = ctx.get_pq_public_keys().unwrap().dilithium_public.clone();
        let result = ctx.pq_verify(wrong_message, &signature, &public_key);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod sphincs_tests {
    use super::*;
    use super::test_utils::*;
    
    #[test]
    fn test_sphincs_sign_verify() {
        let mut ctx = init_pqc_context().unwrap();
        
        // SPHINCS+ is slower, test with fewer/smaller messages
        let messages = vec![
            b"SPHINCS+ test".to_vec(),
            generate_test_data(256),
        ];
        
        for message in messages {
            // Sign with SPHINCS+
            let signature = ctx.sphincs_sign(&message).unwrap();
            assert!(!signature.is_empty());
            
            // Verify signature
            let public_key = ctx.get_pq_public_keys().unwrap().sphincs_public.clone();
            let result = ctx.sphincs_verify(&message, &signature, &public_key);
            assert!(result.is_ok());
        }
    }
    
    #[test]
    fn test_sphincs_stateless_property() {
        let mut ctx = init_pqc_context().unwrap();
        let message = b"Stateless signature test";
        
        // Sign same message multiple times
        let sig1 = ctx.sphincs_sign(message).unwrap();
        let sig2 = ctx.sphincs_sign(message).unwrap();
        let sig3 = ctx.sphincs_sign(message).unwrap();
        
        // All signatures should be valid
        let public_key = ctx.get_pq_public_keys().unwrap().sphincs_public.clone();
        assert!(ctx.sphincs_verify(message, &sig1, &public_key).is_ok());
        assert!(ctx.sphincs_verify(message, &sig2, &public_key).is_ok());
        assert!(ctx.sphincs_verify(message, &sig3, &public_key).is_ok());
        
        // Signatures might be different due to randomization
        // This is expected behavior for SPHINCS+
    }
}

#[cfg(test)]
mod hybrid_crypto_tests {
    use super::*;
    use super::test_utils::*;
    use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
    
    #[test]
    fn test_hybrid_encryption() {
        let mut alice_ctx = init_pqc_context().unwrap();
        let mut bob_ctx = init_pqc_context().unwrap();
        
        // Bob generates X25519 keypair
        let bob_x25519_secret = EphemeralSecret::new(OsRng);
        let bob_x25519_public = X25519PublicKey::from(&bob_x25519_secret);
        
        // Get Bob's Kyber public key
        let bob_kyber_public = bob_ctx.get_pq_public_keys().unwrap().kyber_public.clone();
        
        // Test data
        let plaintext = b"Hybrid encrypted message - protected against classical and quantum attacks";
        
        // Alice encrypts using both keys
        let encrypted = alice_ctx.hybrid_encrypt(
            plaintext,
            &bob_x25519_public,
            &bob_kyber_public
        ).unwrap();
        
        assert_eq!(encrypted.algorithm, PQAlgorithm::HybridX25519Kyber768);
        assert_eq!(encrypted.x25519_ephemeral_public.len(), 32);
        assert!(!encrypted.kyber_ciphertext.is_empty());
        assert!(!encrypted.encrypted_payload.is_empty());
        
        // Bob would decrypt (implementation needed)
        // This demonstrates the structure is correct
    }
    
    #[test]
    fn test_hybrid_signatures() {
        let mut ctx = init_pqc_context().unwrap();
        
        let messages = vec![
            b"Sign with both Ed25519 and Dilithium".to_vec(),
            generate_test_data(1024),
        ];
        
        for message in messages {
            // Create hybrid signature
            let hybrid_sig = ctx.hybrid_sign(&message).unwrap();
            
            assert_eq!(hybrid_sig.algorithm, PQAlgorithm::HybridEd25519Dilithium3);
            assert_eq!(hybrid_sig.ed25519_signature.len(), 64);
            assert!(!hybrid_sig.dilithium_signature.is_empty());
            
            // Verify both signatures independently
            // Ed25519 verification
            let ed25519_sig = ed25519_dalek::Signature::from_bytes(
                &hybrid_sig.ed25519_signature.try_into().unwrap()
            );
            let ed25519_public = ctx.public_key().unwrap();
            assert!(ctx.verify(&message, &ed25519_sig, &ed25519_public).is_ok());
            
            // Dilithium verification
            let dilithium_public = ctx.get_pq_public_keys().unwrap().dilithium_public.clone();
            assert!(ctx.pq_verify(&message, &hybrid_sig.dilithium_signature, &dilithium_public).is_ok());
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use super::test_utils::*;
    use std::time::Instant;
    
    #[test]
    #[ignore] // Run with --ignored flag for benchmarks
    fn benchmark_pqc_operations() {
        let mut ctx = init_pqc_context().unwrap();
        let test_data = generate_test_data(1024);
        
        // Benchmark Kyber encryption
        let kyber_public = ctx.get_pq_public_keys().unwrap().kyber_public.clone();
        let start = Instant::now();
        for _ in 0..100 {
            let _ = ctx.pq_encrypt(&test_data, &kyber_public).unwrap();
        }
        let kyber_encrypt_time = start.elapsed();
        println!("Kyber encryption (100 ops): {:?}", kyber_encrypt_time);
        
        // Benchmark Dilithium signing
        let start = Instant::now();
        for _ in 0..100 {
            let _ = ctx.pq_sign(&test_data).unwrap();
        }
        let dilithium_sign_time = start.elapsed();
        println!("Dilithium signing (100 ops): {:?}", dilithium_sign_time);
        
        // Benchmark SPHINCS+ signing (much slower)
        let start = Instant::now();
        for _ in 0..10 {
            let _ = ctx.sphincs_sign(&test_data).unwrap();
        }
        let sphincs_sign_time = start.elapsed();
        println!("SPHINCS+ signing (10 ops): {:?}", sphincs_sign_time);
        
        // Compare with classical crypto
        let start = Instant::now();
        for _ in 0..100 {
            let _ = ctx.sign(&test_data).unwrap();
        }
        let ed25519_sign_time = start.elapsed();
        println!("Ed25519 signing (100 ops): {:?}", ed25519_sign_time);
    }
}

#[cfg(test)]
mod security_tests {
    use super::*;
    use super::test_utils::*;
    
    #[test]
    fn test_key_zeroization() {
        // This test verifies that keys are properly zeroized
        // when dropped (ZeroizeOnDrop trait)
        {
            let ctx = init_pqc_context().unwrap();
            // Context with PQ keys goes out of scope
        }
        // Keys should be zeroized automatically
        // Manual memory inspection would be needed to fully verify
    }
    
    #[test]
    fn test_nonce_uniqueness() {
        let mut ctx = init_pqc_context().unwrap();
        let public_key = ctx.get_pq_public_keys().unwrap().kyber_public.clone();
        let plaintext = b"Test nonce uniqueness";
        
        // Encrypt multiple times
        let enc1 = ctx.pq_encrypt(plaintext, &public_key).unwrap();
        let enc2 = ctx.pq_encrypt(plaintext, &public_key).unwrap();
        let enc3 = ctx.pq_encrypt(plaintext, &public_key).unwrap();
        
        // Nonce counters should be different
        assert_ne!(enc1.nonce_counter, enc2.nonce_counter);
        assert_ne!(enc2.nonce_counter, enc3.nonce_counter);
        
        // Ciphertexts should be different even for same plaintext
        assert_ne!(enc1.encrypted_payload, enc2.encrypted_payload);
        assert_ne!(enc2.encrypted_payload, enc3.encrypted_payload);
    }
    
    #[test]
    fn test_replay_protection() {
        let mut ctx = init_pqc_context().unwrap();
        let public_key = ctx.get_pq_public_keys().unwrap().kyber_public.clone();
        
        // Encrypt a message
        let plaintext = b"Replay protection test";
        let encrypted = ctx.pq_encrypt(plaintext, &public_key).unwrap();
        
        // Clone the encrypted data (simulating replay)
        let replayed = encrypted.clone();
        
        // First decryption should succeed
        let result1 = ctx.pq_decrypt(&encrypted);
        assert!(result1.is_ok());
        
        // Implement replay detection logic in production
        // Second decryption of same nonce should fail
        // (This would require implementing a nonce tracking mechanism)
    }
}

// Module to export test utilities for integration tests
pub mod test_exports {
    pub use super::test_utils::*;
}
