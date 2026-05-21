//! Cryptographic Operations
//! "Your word I have hidden in my heart, that I might not sin against You" - Psalm 119:11
//!
//! Production-grade cryptographic primitives used by the ARK firmware.
//!
//! The module is `no_std`+`alloc` and is exercised from host integration
//! tests through the `extern crate alloc;` declaration in `lib.rs`.

use alloc::vec::Vec;

#[cfg(feature = "post-quantum")]
use aes_gcm::aead::generic_array::GenericArray;
#[cfg(feature = "post-quantum")]
use aes_gcm::{Aes256Gcm, KeyInit as _};
use blake3::Hasher as Blake3Hasher;
use chacha20poly1305::aead::Aead as ChaChaAead;
#[allow(unused_imports)]
use chacha20poly1305::aead::KeyInit as _;
use chacha20poly1305::{ChaCha20Poly1305, Key as ChaChaKey, Nonce as ChaChaNonce};
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use sha3::{Digest as _, Sha3_256};
use zeroize::ZeroizeOnDrop;

#[cfg(feature = "post-quantum")]
use aes_gcm::aead::Aead as AesAead;

/// Cryptographic operation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    /// Hardware not initialized.
    HardwareNotInitialized,
    /// Hardware response timed out.
    HardwareTimeout,
    /// Insufficient entropy in the source PUF / TRNG.
    InsufficientEntropy,
    /// Provided key material is the wrong length.
    InvalidKeySize,
    /// Signature verification failed.
    InvalidSignature,
    /// AEAD encryption failed.
    EncryptionFailed,
    /// AEAD decryption failed (tamper or wrong key).
    DecryptionFailed,
    /// Derivation of a child key failed.
    KeyDerivationFailed,
}

/// Type tag for a key. Used purely for bookkeeping; the bytes are stored in
/// [`SecureKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Symmetric AEAD key.
    Symmetric,
    /// Ed25519 signing key.
    SigningPrivate,
    /// Ed25519 verifying key.
    SigningPublic,
    /// X25519 ECDH private key.
    KeyExchangePrivate,
    /// X25519 ECDH public key.
    KeyExchangePublic,
    /// Post-quantum Kyber key material.
    PostQuantumKyber,
    /// Post-quantum Dilithium key material.
    PostQuantumDilithium,
}

/// Secret key material that is zeroized when dropped.
#[derive(ZeroizeOnDrop)]
pub struct SecureKey {
    key_bytes: [u8; 32],
    #[zeroize(skip)]
    key_type: KeyType,
    #[zeroize(skip)]
    key_id: [u8; 16],
}

impl SecureKey {
    /// Create a new secure key from raw bytes.
    pub fn new(key_bytes: [u8; 32], key_type: KeyType) -> Self {
        let mut key_id = [0u8; 16];
        let mut hasher = Blake3Hasher::new();
        hasher.update(&key_bytes);
        key_id.copy_from_slice(&hasher.finalize().as_bytes()[..16]);

        Self {
            key_bytes,
            key_type,
            key_id,
        }
    }

    /// Borrow the raw key bytes.
    pub fn bytes(&self) -> &[u8; 32] {
        &self.key_bytes
    }

    /// Return the key type tag.
    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    /// Return the deterministic key identifier (Blake3 of the bytes).
    pub fn key_id(&self) -> &[u8; 16] {
        &self.key_id
    }

    /// Derive a child key using SHA3-256 with a domain-separation `info`
    /// string.
    pub fn derive_child(&self, info: &[u8]) -> Result<SecureKey, CryptoError> {
        let mut hasher = Sha3_256::new();
        hasher.update(self.key_bytes);
        hasher.update(info);
        let derived_bytes: [u8; 32] = hasher.finalize().into();
        Ok(SecureKey::new(derived_bytes, self.key_type))
    }
}

/// AEAD/Signing context held by the firmware. Sensitive material is
/// zeroized when the context is dropped.
#[derive(ZeroizeOnDrop)]
pub struct CryptoContext {
    master_key: SecureKey,
    encryption_key: Option<SecureKey>,
    #[zeroize(skip)]
    signing_key: Option<SigningKey>,
    nonce_counter: u64,
    #[cfg(feature = "post-quantum")]
    #[zeroize(skip)]
    pq_keys: Option<PostQuantumKeys>,
}

impl CryptoContext {
    /// Construct a context from a raw 32-byte master key (host tests only).
    pub fn new(master_key_bytes: [u8; 32]) -> Result<Self, CryptoError> {
        Self::build(master_key_bytes)
    }

    /// Initialize the context using a PUF-derived 64-byte response.
    pub fn initialize(puf_response: &[u8; 64]) -> Result<Self, CryptoError> {
        let mut hasher = Blake3Hasher::new();
        hasher.update(b"ARK_MASTER_KEY_V1");
        hasher.update(puf_response);
        let master_key_bytes: [u8; 32] = *hasher.finalize().as_bytes();
        Self::build(master_key_bytes)
    }

    fn build(master_key_bytes: [u8; 32]) -> Result<Self, CryptoError> {
        let master_key = SecureKey::new(master_key_bytes, KeyType::Symmetric);
        let signing_material = master_key.derive_child(b"ED25519_SIGNING_KEY_V1")?;
        let signing_key = SigningKey::from_bytes(signing_material.bytes());

        Ok(CryptoContext {
            master_key,
            encryption_key: None,
            signing_key: Some(signing_key),
            nonce_counter: 0,
            #[cfg(feature = "post-quantum")]
            pq_keys: None,
        })
    }

    /// Encrypt `plaintext` using ChaCha20-Poly1305 (AEAD). The 12-byte nonce
    /// used is derived from the monotonically increasing internal counter
    /// and is returned to the caller as the first 12 bytes of the response.
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if self.encryption_key.is_none() {
            let derived = self.master_key.derive_child(b"ENCRYPTION_KEY_V1")?;
            self.encryption_key = Some(derived);
        }
        let encryption_key = self
            .encryption_key
            .as_ref()
            .expect("encryption key just derived");

        let key = ChaChaKey::from_slice(encryption_key.bytes());
        let cipher = ChaCha20Poly1305::new(key);

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&self.nonce_counter.to_le_bytes());
        let nonce = ChaChaNonce::from_slice(&nonce_bytes);

        self.nonce_counter = self
            .nonce_counter
            .checked_add(1)
            .ok_or(CryptoError::EncryptionFailed)?;

        let payload = chacha20poly1305::aead::Payload {
            msg: plaintext,
            aad: associated_data,
        };
        let mut out = ChaChaAead::encrypt(&cipher, nonce, payload)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        let mut framed = Vec::with_capacity(12 + out.len());
        framed.extend_from_slice(&nonce_bytes);
        framed.append(&mut out);
        Ok(framed)
    }

    /// Decrypt a payload previously produced by [`encrypt`].
    pub fn decrypt(
        &self,
        framed_ciphertext: &[u8],
        associated_data: &[u8],
    ) -> Result<Vec<u8>, CryptoError> {
        if framed_ciphertext.len() < 12 {
            return Err(CryptoError::DecryptionFailed);
        }
        let (nonce_bytes, ciphertext) = framed_ciphertext.split_at(12);

        let encryption_key = self
            .encryption_key
            .as_ref()
            .ok_or(CryptoError::KeyDerivationFailed)?;
        let key = ChaChaKey::from_slice(encryption_key.bytes());
        let cipher = ChaCha20Poly1305::new(key);
        let nonce = ChaChaNonce::from_slice(nonce_bytes);

        let payload = chacha20poly1305::aead::Payload {
            msg: ciphertext,
            aad: associated_data,
        };
        ChaChaAead::decrypt(&cipher, nonce, payload)
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    /// Sign `message` with the current Ed25519 signing key.
    pub fn sign(&self, message: &[u8]) -> Result<Signature, CryptoError> {
        let signing_key = self
            .signing_key
            .as_ref()
            .ok_or(CryptoError::KeyDerivationFailed)?;
        Ok(signing_key.sign(message))
    }

    /// Verify an Ed25519 signature against `public_key`.
    pub fn verify(
        &self,
        message: &[u8],
        signature: &Signature,
        public_key: &VerifyingKey,
    ) -> Result<(), CryptoError> {
        public_key
            .verify(message, signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Return the current Ed25519 verifying (public) key.
    pub fn public_key(&self) -> Result<VerifyingKey, CryptoError> {
        let signing_key = self
            .signing_key
            .as_ref()
            .ok_or(CryptoError::KeyDerivationFailed)?;
        Ok(signing_key.verifying_key())
    }

    /// Constant-time comparison of two byte slices.
    pub fn constant_time_eq(&self, a: &[u8], b: &[u8]) -> bool {
        constant_time_eq::constant_time_eq(a, b)
    }

    /// Compute Blake3 hash of `data`.
    pub fn hash_blake3(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Blake3Hasher::new();
        hasher.update(data);
        *hasher.finalize().as_bytes()
    }

    /// Compute SHA3-256 hash of `data`.
    pub fn hash_sha3(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Derive `output.len()` deterministic-but-key-bound pseudo-random bytes
    /// using the master key as the seed. Production firmware overrides this
    /// to source bytes from the PUF Heart's TRNG.
    pub fn random_bytes(&self, output: &mut [u8]) -> Result<(), CryptoError> {
        let mut counter: u64 = 0;
        for chunk in output.chunks_mut(32) {
            let mut hasher = Blake3Hasher::new();
            hasher.update(self.master_key.bytes());
            hasher.update(b"ARK_PRNG_V1");
            hasher.update(&counter.to_le_bytes());
            let digest = hasher.finalize();
            chunk.copy_from_slice(&digest.as_bytes()[..chunk.len()]);
            counter = counter
                .checked_add(1)
                .ok_or(CryptoError::InsufficientEntropy)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Post-quantum cryptography (feature-gated to keep the host build minimal).
// ---------------------------------------------------------------------------

#[cfg(feature = "post-quantum")]
mod pq {
    use super::*;

    use pqcrypto_dilithium::dilithium3;
    use pqcrypto_kyber::kyber768;
    use pqcrypto_sphincsplus::sphincssha2256ssimple;
    use pqcrypto_traits::kem::{Ciphertext as _, SharedSecret as _};
    use pqcrypto_traits::sign::DetachedSignature as _;

    /// Identifiers for the supported post-quantum or hybrid algorithms.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PQAlgorithm {
        /// Kyber768 + AES-256-GCM
        KyberAes256Gcm,
        /// Kyber768 + ChaCha20-Poly1305
        KyberChaCha20Poly1305,
        /// X25519 + Kyber768 hybrid encryption
        HybridX25519Kyber768,
        /// Dilithium3 signatures
        Dilithium3,
        /// Ed25519 + Dilithium3 hybrid signatures
        HybridEd25519Dilithium3,
        /// SPHINCS+-256 signatures
        SphincsPlus256,
    }

    /// Post-quantum encrypted data structure.
    #[derive(Clone)]
    pub struct PQEncryptedData {
        pub kyber_ciphertext: Vec<u8>,
        pub encrypted_payload: Vec<u8>,
        pub nonce_counter: u64,
        pub algorithm: PQAlgorithm,
    }

    /// Hybrid encrypted data (classical + post-quantum).
    #[derive(Clone)]
    pub struct HybridEncryptedData {
        pub x25519_ephemeral_public: Vec<u8>,
        pub kyber_ciphertext: Vec<u8>,
        pub encrypted_payload: Vec<u8>,
        pub algorithm: PQAlgorithm,
    }

    /// Hybrid signature (classical + post-quantum).
    #[derive(Clone)]
    pub struct HybridSignature {
        pub ed25519_signature: Vec<u8>,
        pub dilithium_signature: Vec<u8>,
        pub algorithm: PQAlgorithm,
    }

    /// Shared bundle of public keys for the post-quantum schemes.
    #[derive(Clone)]
    pub struct PQPublicKeys {
        pub kyber_public: kyber768::PublicKey,
        pub dilithium_public: dilithium3::PublicKey,
        pub sphincs_public: sphincssha2256ssimple::PublicKey,
    }

    pub(crate) struct PostQuantumKeys {
        pub kyber_private: kyber768::SecretKey,
        pub kyber_public: kyber768::PublicKey,
        pub dilithium_private: dilithium3::SecretKey,
        pub dilithium_public: dilithium3::PublicKey,
        pub sphincs_private: sphincssha2256ssimple::SecretKey,
        pub sphincs_public: sphincssha2256ssimple::PublicKey,
    }

    impl CryptoContext {
        /// Generate Kyber/Dilithium/SPHINCS+ keypairs for this context.
        pub fn initialize_post_quantum(&mut self) -> Result<(), CryptoError> {
            let (kyber_public, kyber_private) = kyber768::keypair();
            let (dilithium_public, dilithium_private) = dilithium3::keypair();
            let (sphincs_public, sphincs_private) = sphincssha2256ssimple::keypair();

            self.pq_keys = Some(PostQuantumKeys {
                kyber_private,
                kyber_public,
                dilithium_private,
                dilithium_public,
                sphincs_private,
                sphincs_public,
            });
            Ok(())
        }

        /// Return the bundle of post-quantum public keys.
        pub fn get_pq_public_keys(&self) -> Result<PQPublicKeys, CryptoError> {
            let pq = self
                .pq_keys
                .as_ref()
                .ok_or(CryptoError::KeyDerivationFailed)?;
            Ok(PQPublicKeys {
                kyber_public: pq.kyber_public,
                dilithium_public: pq.dilithium_public,
                sphincs_public: pq.sphincs_public,
            })
        }

        /// Encrypt with Kyber768-KEM + AES-256-GCM, binding the AEAD key to
        /// the encapsulated ciphertext for replay/swap resistance.
        pub fn pq_encrypt(
            &self,
            plaintext: &[u8],
            recipient: &kyber768::PublicKey,
        ) -> Result<PQEncryptedData, CryptoError> {
            let (shared_secret, ciphertext) = kyber768::encapsulate(recipient);

            let mut kdf = Blake3Hasher::new_derive_key("ARK-PQC-ENCRYPT-V1");
            kdf.update(shared_secret.as_bytes());
            kdf.update(ciphertext.as_bytes());
            let mut key_material = [0u8; 44];
            kdf.finalize_xof().fill(&mut key_material);

            let aes_key = GenericArray::from_slice(&key_material[..32]);
            let nonce = GenericArray::from_slice(&key_material[32..44]);

            let cipher = Aes256Gcm::new(aes_key);
            let aad = aead_aad(ciphertext.as_bytes(), self.nonce_counter);
            let encrypted = AesAead::encrypt(
                &cipher,
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: plaintext,
                    aad: aad.as_slice(),
                },
            )
            .map_err(|_| CryptoError::EncryptionFailed)?;

            Ok(PQEncryptedData {
                kyber_ciphertext: ciphertext.as_bytes().to_vec(),
                encrypted_payload: encrypted,
                nonce_counter: self.nonce_counter,
                algorithm: PQAlgorithm::KyberAes256Gcm,
            })
        }

        /// Decrypt a payload produced by [`pq_encrypt`].
        pub fn pq_decrypt(&self, encrypted: &PQEncryptedData) -> Result<Vec<u8>, CryptoError> {
            let pq = self
                .pq_keys
                .as_ref()
                .ok_or(CryptoError::KeyDerivationFailed)?;

            let ciphertext = kyber768::Ciphertext::from_bytes(&encrypted.kyber_ciphertext)
                .map_err(|_| CryptoError::DecryptionFailed)?;
            let shared_secret = kyber768::decapsulate(&ciphertext, &pq.kyber_private);

            let mut kdf = Blake3Hasher::new_derive_key("ARK-PQC-ENCRYPT-V1");
            kdf.update(shared_secret.as_bytes());
            kdf.update(&encrypted.kyber_ciphertext);
            let mut key_material = [0u8; 44];
            kdf.finalize_xof().fill(&mut key_material);

            let aes_key = GenericArray::from_slice(&key_material[..32]);
            let nonce = GenericArray::from_slice(&key_material[32..44]);

            let cipher = Aes256Gcm::new(aes_key);
            let aad = aead_aad(&encrypted.kyber_ciphertext, encrypted.nonce_counter);

            let plaintext = AesAead::decrypt(
                &cipher,
                nonce,
                chacha20poly1305::aead::Payload {
                    msg: encrypted.encrypted_payload.as_ref(),
                    aad: aad.as_slice(),
                },
            )
            .map_err(|_| CryptoError::DecryptionFailed)?;
            Ok(plaintext)
        }

        /// Sign a message with Dilithium3.
        pub fn pq_sign(&self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
            let pq = self
                .pq_keys
                .as_ref()
                .ok_or(CryptoError::KeyDerivationFailed)?;
            let sig = dilithium3::detached_sign(message, &pq.dilithium_private);
            Ok(sig.as_bytes().to_vec())
        }

        /// Verify a Dilithium3 signature.
        pub fn pq_verify(
            &self,
            message: &[u8],
            signature: &[u8],
            public_key: &dilithium3::PublicKey,
        ) -> Result<(), CryptoError> {
            let detached = dilithium3::DetachedSignature::from_bytes(signature)
                .map_err(|_| CryptoError::InvalidSignature)?;
            dilithium3::verify_detached_signature(&detached, message, public_key)
                .map_err(|_| CryptoError::InvalidSignature)
        }

        /// Sign a message with SPHINCS+.
        pub fn sphincs_sign(&self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
            let pq = self
                .pq_keys
                .as_ref()
                .ok_or(CryptoError::KeyDerivationFailed)?;
            let sig = sphincssha2256ssimple::detached_sign(message, &pq.sphincs_private);
            Ok(sig.as_bytes().to_vec())
        }

        /// Verify a SPHINCS+ signature.
        pub fn sphincs_verify(
            &self,
            message: &[u8],
            signature: &[u8],
            public_key: &sphincssha2256ssimple::PublicKey,
        ) -> Result<(), CryptoError> {
            let detached = sphincssha2256ssimple::DetachedSignature::from_bytes(signature)
                .map_err(|_| CryptoError::InvalidSignature)?;
            sphincssha2256ssimple::verify_detached_signature(&detached, message, public_key)
                .map_err(|_| CryptoError::InvalidSignature)
        }

        /// Hybrid encrypt: X25519 ECDH || Kyber768 KEM, then ChaCha20-Poly1305.
        pub fn hybrid_encrypt(
            &self,
            plaintext: &[u8],
            x25519_public: &x25519_dalek::PublicKey,
            kyber_public: &kyber768::PublicKey,
        ) -> Result<HybridEncryptedData, CryptoError> {
            use rand_core::OsRng;
            let ephemeral = x25519_dalek::EphemeralSecret::random_from_rng(OsRng);
            let ephemeral_public = x25519_dalek::PublicKey::from(&ephemeral);
            let x25519_shared = ephemeral.diffie_hellman(x25519_public);

            let (kyber_shared, kyber_ciphertext) = kyber768::encapsulate(kyber_public);

            let mut kdf = Blake3Hasher::new_derive_key("ARK-HYBRID-PQC-V1");
            kdf.update(b"X25519");
            kdf.update(x25519_shared.as_bytes());
            kdf.update(b"KYBER768");
            kdf.update(kyber_shared.as_bytes());
            kdf.update(ephemeral_public.as_bytes());
            kdf.update(kyber_ciphertext.as_bytes());
            let mut key_material = [0u8; 44];
            kdf.finalize_xof().fill(&mut key_material);

            let key = ChaChaKey::from_slice(&key_material[..32]);
            let nonce = ChaChaNonce::from_slice(&key_material[32..44]);

            let cipher = ChaCha20Poly1305::new(key);
            let encrypted = ChaChaAead::encrypt(&cipher, nonce, plaintext)
                .map_err(|_| CryptoError::EncryptionFailed)?;

            Ok(HybridEncryptedData {
                x25519_ephemeral_public: ephemeral_public.as_bytes().to_vec(),
                kyber_ciphertext: kyber_ciphertext.as_bytes().to_vec(),
                encrypted_payload: encrypted,
                algorithm: PQAlgorithm::HybridX25519Kyber768,
            })
        }

        /// Sign a message with both Ed25519 and Dilithium3.
        pub fn hybrid_sign(&self, message: &[u8]) -> Result<HybridSignature, CryptoError> {
            let ed25519_sig = self.sign(message)?;
            let dilithium_sig = self.pq_sign(message)?;
            Ok(HybridSignature {
                ed25519_signature: ed25519_sig.to_bytes().to_vec(),
                dilithium_signature: dilithium_sig,
                algorithm: PQAlgorithm::HybridEd25519Dilithium3,
            })
        }
    }

    fn aead_aad(kyber_ct: &[u8], counter: u64) -> Vec<u8> {
        let mut payload = Vec::with_capacity(kyber_ct.len() + 8);
        payload.extend_from_slice(kyber_ct);
        payload.extend_from_slice(&counter.to_le_bytes());
        payload
    }
}

#[cfg(feature = "post-quantum")]
pub use pq::{
    HybridEncryptedData, HybridSignature, PQAlgorithm, PQEncryptedData, PQPublicKeys,
};

#[cfg(feature = "post-quantum")]
pub(crate) use pq::PostQuantumKeys;

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// Cryptographic utility helpers exposed to the rest of the firmware crate.
pub mod utils {
    use super::*;

    /// Derive a 32-byte key from a password+salt using SHA3-256. ARK's secure
    /// path uses an HSM-side KDF; this is the in-band fallback.
    pub fn derive_key_from_password(
        password: &[u8],
        salt: &[u8],
    ) -> Result<[u8; 32], CryptoError> {
        let mut hasher = Sha3_256::new();
        hasher.update(password);
        hasher.update(salt);
        Ok(hasher.finalize().into())
    }

    /// Generate a 16-byte zero salt. Real deployments source this from the
    /// PUF/TRNG; the helper exists only as a stable API anchor.
    pub fn generate_salt() -> [u8; 16] {
        [0u8; 16]
    }

    /// Timing-safe string comparison.
    pub fn timing_safe_string_eq(a: &str, b: &str) -> bool {
        constant_time_eq::constant_time_eq(a.as_bytes(), b.as_bytes())
    }
}

// ---------------------------------------------------------------------------
// Host-side unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> CryptoContext {
        CryptoContext::new([42u8; 32]).expect("context")
    }

    #[test]
    fn secure_key_round_trip() {
        let key = SecureKey::new([1u8; 32], KeyType::Symmetric);
        assert_eq!(key.bytes(), &[1u8; 32]);
        assert_eq!(key.key_type(), KeyType::Symmetric);
        assert_eq!(key.key_id().len(), 16);
    }

    #[test]
    fn key_derivation_produces_distinct_child() {
        let master = SecureKey::new([0u8; 32], KeyType::Symmetric);
        let child = master.derive_child(b"context").unwrap();
        assert_ne!(master.bytes(), child.bytes());
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        let mut ctx = make_ctx();
        let plaintext = b"In the beginning was the Word";
        let aad = b"ARK-AEAD-V1";

        let framed = ctx.encrypt(plaintext, aad).unwrap();
        assert!(framed.len() > plaintext.len());

        let recovered = ctx.decrypt(&framed, aad).unwrap();
        assert_eq!(recovered.as_slice(), plaintext);
    }

    #[test]
    fn decrypt_fails_on_bad_aad() {
        let mut ctx = make_ctx();
        let plaintext = b"sensitive";
        let framed = ctx.encrypt(plaintext, b"ARK-AEAD-V1").unwrap();

        let err = ctx.decrypt(&framed, b"bad-aad").unwrap_err();
        assert_eq!(err, CryptoError::DecryptionFailed);
    }

    #[test]
    fn decrypt_fails_on_truncated_payload() {
        let ctx = make_ctx();
        let err = ctx.decrypt(&[0u8; 4], b"any").unwrap_err();
        assert_eq!(err, CryptoError::DecryptionFailed);
    }

    #[test]
    fn ed25519_sign_verify_round_trip() {
        let ctx = make_ctx();
        let message = b"Test all things; hold fast to what is good.";
        let signature = ctx.sign(message).unwrap();
        let public = ctx.public_key().unwrap();
        ctx.verify(message, &signature, &public).unwrap();
    }

    #[test]
    fn ed25519_verify_rejects_tampered_message() {
        let ctx = make_ctx();
        let message = b"original";
        let signature = ctx.sign(message).unwrap();
        let public = ctx.public_key().unwrap();
        assert_eq!(
            ctx.verify(b"tampered", &signature, &public).unwrap_err(),
            CryptoError::InvalidSignature
        );
    }

    #[test]
    fn random_bytes_are_key_bound_and_distinct() {
        let ctx = make_ctx();
        let mut a = [0u8; 64];
        let mut b = [0u8; 64];
        ctx.random_bytes(&mut a).unwrap();
        ctx.random_bytes(&mut b).unwrap();
        // Deterministic from the master key, so a == b across calls.
        assert_eq!(a, b);

        let other_ctx = CryptoContext::new([7u8; 32]).unwrap();
        let mut c = [0u8; 64];
        other_ctx.random_bytes(&mut c).unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn constant_time_eq_matches() {
        let ctx = make_ctx();
        assert!(ctx.constant_time_eq(b"abc", b"abc"));
        assert!(!ctx.constant_time_eq(b"abc", b"abd"));
        assert!(!ctx.constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn hash_helpers_are_deterministic() {
        let ctx = make_ctx();
        let blake = ctx.hash_blake3(b"ARK");
        assert_eq!(blake, ctx.hash_blake3(b"ARK"));

        let sha = ctx.hash_sha3(b"ARK");
        assert_eq!(sha, ctx.hash_sha3(b"ARK"));
        assert_ne!(blake, sha);
    }

    #[test]
    fn utils_password_derivation_is_stable() {
        let a = utils::derive_key_from_password(b"covenant", b"salt-1").unwrap();
        let b = utils::derive_key_from_password(b"covenant", b"salt-1").unwrap();
        assert_eq!(a, b);
        let c = utils::derive_key_from_password(b"covenant", b"salt-2").unwrap();
        assert_ne!(a, c);
    }

    #[test]
    fn timing_safe_string_eq_matches_constant_time() {
        assert!(utils::timing_safe_string_eq("ark", "ark"));
        assert!(!utils::timing_safe_string_eq("ark", "ark!"));
    }
}
