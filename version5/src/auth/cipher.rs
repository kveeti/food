use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use anyhow::{Result, anyhow};
use rand::{RngCore, rngs::OsRng};

pub struct TokenCipher(Aes256Gcm);

impl TokenCipher {
    pub fn new(key: &[u8; 32]) -> Self {
        Self(Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key)))
    }

    pub fn encrypt(&self, value: &str, associated_data: &str) -> Result<Vec<u8>> {
        let mut nonce = [0; 12];
        OsRng.fill_bytes(&mut nonce);
        let ciphertext = self
            .0
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: value.as_bytes(),
                    aad: associated_data.as_bytes(),
                },
            )
            .map_err(|_| anyhow!("stored token encryption failed"))?;

        Ok([nonce.as_slice(), ciphertext.as_slice()].concat())
    }

    pub fn decrypt(&self, value: &[u8], associated_data: &str) -> Result<String> {
        let (nonce, ciphertext) = value
            .split_at_checked(12)
            .ok_or_else(|| anyhow!("stored token is too short"))?;
        let plaintext = self
            .0
            .decrypt(
                Nonce::from_slice(nonce),
                Payload {
                    msg: ciphertext,
                    aad: associated_data.as_bytes(),
                },
            )
            .map_err(|_| {
                anyhow!("stored token decryption failed because its key or encrypted value does not match")
            })?;

        String::from_utf8(plaintext).map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::TokenCipher;

    #[test]
    fn tokens_are_bound_to_their_session_and_kind() {
        let cipher = TokenCipher::new(&[0; 32]);
        let encrypted = cipher.encrypt("secret", "session-1:access").unwrap();

        assert_eq!(
            cipher.decrypt(&encrypted, "session-1:access").unwrap(),
            "secret"
        );
        assert!(cipher.decrypt(&encrypted, "session-2:access").is_err());
        assert!(cipher.decrypt(&encrypted, "session-1:refresh").is_err());
    }
}
