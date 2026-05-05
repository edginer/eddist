use std::sync::OnceLock;

use base64::Engine;
use chacha20poly1305::{KeyInit, aead::Aead};

static KEY: OnceLock<Vec<u8>> = OnceLock::new();

fn derived_key() -> &'static [u8] {
    KEY.get_or_init(|| {
        std::env::var("TINKER_SECRET")
            .expect("TINKER_SECRET must be set")
            .as_bytes()
            .iter()
            .take(32)
            .copied()
            .collect()
    })
}

/// Encrypts `plain` with ChaCha20-Poly1305 using a random nonce derived from TINKER_SECRET.
/// Returns a `v1:<base64(nonce || ciphertext)>` string.
pub fn encrypt(plain: &str) -> String {
    let key = derived_key();
    let nonce_bytes: [u8; 12] = rand::random();
    let ciphertext = chacha20poly1305::ChaCha20Poly1305::new(
        md5::digest::generic_array::GenericArray::from_slice(key),
    )
    .encrypt(
        chacha20poly1305::Nonce::from_slice(&nonce_bytes),
        chacha20poly1305::aead::Payload {
            msg: plain.as_bytes(),
            aad: b"",
        },
    )
    .expect("encryption failed");

    let mut payload = nonce_bytes.to_vec();
    payload.extend_from_slice(&ciphertext);
    format!(
        "v1:{}",
        base64::engine::general_purpose::STANDARD.encode(&payload)
    )
}

/// Decrypts a `v1:<base64(nonce || ciphertext)>` string produced by [`encrypt`].
pub fn decrypt(b64: &str) -> anyhow::Result<String> {
    let b64 = b64
        .strip_prefix("v1:")
        .ok_or_else(|| anyhow::anyhow!("unknown encryption format (missing v1: prefix)"))?;

    let key = derived_key();
    let data = base64::engine::general_purpose::STANDARD.decode(b64)?;
    anyhow::ensure!(data.len() >= 12, "ciphertext too short");

    let (nonce_bytes, ciphertext) = data.split_at(12);
    let plaintext = chacha20poly1305::ChaCha20Poly1305::new(
        md5::digest::generic_array::GenericArray::from_slice(key),
    )
    .decrypt(
        chacha20poly1305::Nonce::from_slice(nonce_bytes),
        chacha20poly1305::aead::Payload {
            msg: ciphertext,
            aad: b"",
        },
    )
    .map_err(|_| anyhow::anyhow!("decryption failed (wrong key or corrupted ciphertext)"))?;

    Ok(std::str::from_utf8(&plaintext)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        unsafe { std::env::set_var("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!") };
        let plain = "my_secret_value";
        let encrypted = encrypt(plain);
        assert!(encrypted.starts_with("v1:"));
        assert_eq!(decrypt(&encrypted).unwrap(), plain);
    }

    #[test]
    fn different_nonces_each_call() {
        unsafe { std::env::set_var("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!") };
        let a = encrypt("same");
        let b = encrypt("same");
        assert_ne!(a, b);
    }

    #[test]
    fn rejects_missing_prefix() {
        unsafe { std::env::set_var("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!") };
        assert!(decrypt("not_v1_prefixed").is_err());
    }
}
