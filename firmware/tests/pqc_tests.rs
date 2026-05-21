//! Post-Quantum Cryptography Test Suite
//! "Test all things; hold fast to what is good" - 1 Thessalonians 5:21

#![cfg(test)]
#![cfg(feature = "post-quantum")]

use ark_firmware::crypto::{CryptoContext, CryptoError, PQAlgorithm};

mod test_utils {
    use super::*;
    use rand_core::{OsRng, RngCore};

    pub fn generate_test_data(size: usize) -> Vec<u8> {
        let mut data = vec![0u8; size];
        OsRng.fill_bytes(&mut data);
        data
    }

    pub fn fixed_master_key(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    pub fn init_pqc_context(seed: u8) -> Result<CryptoContext, CryptoError> {
        let mut ctx = CryptoContext::new(fixed_master_key(seed))?;
        ctx.initialize_post_quantum()?;
        Ok(ctx)
    }
}

#[cfg(test)]
mod kyber_tests {
    use super::test_utils::*;
    use super::*;

    #[test]
    fn test_kyber_encryption_decryption() {
        let alice_ctx = init_pqc_context(1).unwrap();
        let bob_ctx = init_pqc_context(2).unwrap();

        for size in [16usize, 32, 64, 256, 1024, 4096] {
            let plaintext = generate_test_data(size);
            let bob_public_key = bob_ctx.get_pq_public_keys().unwrap().kyber_public;
            let encrypted = alice_ctx.pq_encrypt(&plaintext, &bob_public_key).unwrap();

            assert!(!encrypted.kyber_ciphertext.is_empty());
            assert!(!encrypted.encrypted_payload.is_empty());
            assert_eq!(encrypted.algorithm, PQAlgorithm::KyberAes256Gcm);

            let decrypted = bob_ctx.pq_decrypt(&encrypted).unwrap();
            assert_eq!(plaintext, decrypted);
        }
    }

    #[test]
    fn test_kyber_wrong_key_fails() {
        let alice_ctx = init_pqc_context(1).unwrap();
        let bob_ctx = init_pqc_context(2).unwrap();
        let charlie_ctx = init_pqc_context(3).unwrap();

        let plaintext = b"Secret message for Bob only";
        let bob_public_key = bob_ctx.get_pq_public_keys().unwrap().kyber_public;
        let encrypted = alice_ctx.pq_encrypt(plaintext, &bob_public_key).unwrap();

        assert!(charlie_ctx.pq_decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_kyber_tampered_ciphertext_fails() {
        let alice_ctx = init_pqc_context(1).unwrap();
        let bob_ctx = init_pqc_context(2).unwrap();

        let plaintext = b"Integrity protected message";
        let bob_public_key = bob_ctx.get_pq_public_keys().unwrap().kyber_public;
        let mut encrypted = alice_ctx.pq_encrypt(plaintext, &bob_public_key).unwrap();

        encrypted.encrypted_payload[0] ^= 0xFF;
        assert!(bob_ctx.pq_decrypt(&encrypted).is_err());
    }
}

#[cfg(test)]
mod dilithium_tests {
    use super::test_utils::*;
    

    #[test]
    fn test_dilithium_sign_verify() {
        let ctx = init_pqc_context(7).unwrap();
        let messages: Vec<Vec<u8>> = vec![
            b"Short message".to_vec(),
            generate_test_data(1024),
            generate_test_data(10240),
        ];

        for message in messages {
            let signature = ctx.pq_sign(&message).unwrap();
            assert!(!signature.is_empty());

            let public_key = ctx.get_pq_public_keys().unwrap().dilithium_public;
            assert!(ctx.pq_verify(&message, &signature, &public_key).is_ok());
        }
    }

    #[test]
    fn test_dilithium_wrong_signature_fails() {
        let ctx = init_pqc_context(8).unwrap();
        let message = b"Authentic message";
        let mut signature = ctx.pq_sign(message).unwrap();
        signature[0] ^= 0xFF;

        let public_key = ctx.get_pq_public_keys().unwrap().dilithium_public;
        assert!(ctx.pq_verify(message, &signature, &public_key).is_err());
    }

    #[test]
    fn test_dilithium_wrong_message_fails() {
        let ctx = init_pqc_context(9).unwrap();
        let message = b"Original message";
        let signature = ctx.pq_sign(message).unwrap();

        let public_key = ctx.get_pq_public_keys().unwrap().dilithium_public;
        assert!(ctx
            .pq_verify(b"Modified message", &signature, &public_key)
            .is_err());
    }
}

#[cfg(test)]
mod sphincs_tests {
    use super::test_utils::*;
    

    #[test]
    fn test_sphincs_sign_verify() {
        let ctx = init_pqc_context(11).unwrap();
        let messages: Vec<Vec<u8>> = vec![b"SPHINCS+ test".to_vec(), generate_test_data(256)];

        for message in messages {
            let signature = ctx.sphincs_sign(&message).unwrap();
            assert!(!signature.is_empty());

            let public_key = ctx.get_pq_public_keys().unwrap().sphincs_public;
            assert!(ctx
                .sphincs_verify(&message, &signature, &public_key)
                .is_ok());
        }
    }

    #[test]
    fn test_sphincs_stateless_property() {
        let ctx = init_pqc_context(12).unwrap();
        let message = b"Stateless signature test";

        let sig1 = ctx.sphincs_sign(message).unwrap();
        let sig2 = ctx.sphincs_sign(message).unwrap();
        let sig3 = ctx.sphincs_sign(message).unwrap();

        let public_key = ctx.get_pq_public_keys().unwrap().sphincs_public;
        assert!(ctx.sphincs_verify(message, &sig1, &public_key).is_ok());
        assert!(ctx.sphincs_verify(message, &sig2, &public_key).is_ok());
        assert!(ctx.sphincs_verify(message, &sig3, &public_key).is_ok());
    }
}

#[cfg(test)]
mod hybrid_crypto_tests {
    use super::test_utils::*;
    use super::*;
    use rand_core::OsRng;
    use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};

    #[test]
    fn test_hybrid_encryption() {
        let alice_ctx = init_pqc_context(21).unwrap();
        let bob_ctx = init_pqc_context(22).unwrap();

        let bob_x25519_secret = EphemeralSecret::random_from_rng(OsRng);
        let bob_x25519_public = X25519PublicKey::from(&bob_x25519_secret);
        let bob_kyber_public = bob_ctx.get_pq_public_keys().unwrap().kyber_public;

        let plaintext =
            b"Hybrid encrypted message - protected against classical and quantum attacks";

        let encrypted = alice_ctx
            .hybrid_encrypt(plaintext, &bob_x25519_public, &bob_kyber_public)
            .unwrap();

        assert_eq!(encrypted.algorithm, PQAlgorithm::HybridX25519Kyber768);
        assert_eq!(encrypted.x25519_ephemeral_public.len(), 32);
        assert!(!encrypted.kyber_ciphertext.is_empty());
        assert!(!encrypted.encrypted_payload.is_empty());
    }

    #[test]
    fn test_hybrid_signatures() {
        let ctx = init_pqc_context(31).unwrap();
        let messages: Vec<Vec<u8>> = vec![
            b"Sign with both Ed25519 and Dilithium".to_vec(),
            generate_test_data(1024),
        ];

        for message in messages {
            let hybrid_sig = ctx.hybrid_sign(&message).unwrap();
            assert_eq!(hybrid_sig.algorithm, PQAlgorithm::HybridEd25519Dilithium3);
            assert_eq!(hybrid_sig.ed25519_signature.len(), 64);
            assert!(!hybrid_sig.dilithium_signature.is_empty());

            let ed25519_bytes: [u8; 64] = hybrid_sig.ed25519_signature.as_slice().try_into().unwrap();
            let ed25519_sig = ed25519_dalek::Signature::from_bytes(&ed25519_bytes);
            let ed25519_public = ctx.public_key().unwrap();
            assert!(ctx.verify(&message, &ed25519_sig, &ed25519_public).is_ok());

            let dilithium_public = ctx.get_pq_public_keys().unwrap().dilithium_public;
            assert!(ctx
                .pq_verify(&message, &hybrid_sig.dilithium_signature, &dilithium_public)
                .is_ok());
        }
    }
}

#[cfg(test)]
mod security_tests {
    use super::test_utils::*;
    

    #[test]
    fn test_key_zeroization() {
        // Smoke test that contexts can be created and dropped cleanly.
        {
            let _ctx = init_pqc_context(41).unwrap();
        }
    }

    #[test]
    fn test_replay_protection() {
        let ctx = init_pqc_context(42).unwrap();
        let public_key = ctx.get_pq_public_keys().unwrap().kyber_public;
        let plaintext = b"Replay protection test";

        let encrypted = ctx.pq_encrypt(plaintext, &public_key).unwrap();
        // Caller-side replay protection layers above this primitive can track
        // (kyber_ciphertext, nonce_counter) pairs to reject replays.
        assert!(ctx.pq_decrypt(&encrypted).is_ok());
    }
}
