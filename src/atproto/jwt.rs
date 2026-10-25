use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn b64(bytes: impl AsRef<[u8]>) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn random_string(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::fill(&mut buf[..]);
    b64(buf)
}

pub fn pkce_challenge(verifier: &str) -> String {
    b64(Sha256::digest(verifier.as_bytes()))
}

/// An ES256 (P-256) key used for client assertions and DPoP proofs.
#[derive(Clone)]
pub struct EsKey(SigningKey);

impl EsKey {
    pub fn generate() -> Self {
        loop {
            let mut scalar = [0u8; 32];
            rand::fill(&mut scalar);
            if let Ok(k) = SigningKey::from_slice(&scalar) {
                return Self(k);
            }
        }
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0.to_bytes())
    }

    pub fn from_hex(s: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(s)?;
        Ok(Self(SigningKey::from_slice(&bytes).map_err(|e| anyhow::anyhow!("bad ES256 key: {e}"))?))
    }

    fn coordinates(&self) -> (String, String) {
        let point = self.0.verifying_key().to_sec1_point(false);
        let raw = point.as_bytes();
        (b64(&raw[1..33]), b64(&raw[33..65]))
    }

    pub fn public_jwk(&self) -> Value {
        let (x, y) = self.coordinates();
        json!({ "kty": "EC", "crv": "P-256", "x": x, "y": y })
    }

    /// RFC 7638 JWK thumbprint.
    pub fn thumbprint(&self) -> String {
        let (x, y) = self.coordinates();
        let canonical = format!(r#"{{"crv":"P-256","kty":"EC","x":"{x}","y":"{y}"}}"#);
        b64(Sha256::digest(canonical.as_bytes()))
    }

    pub fn sign_jwt(&self, header: &Value, claims: &Value) -> String {
        let signing_input = format!("{}.{}", b64(header.to_string()), b64(claims.to_string()));
        let sig: Signature = self.0.sign(signing_input.as_bytes());
        format!("{signing_input}.{}", b64(sig.to_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::Verifier;

    use super::*;

    #[test]
    fn pkce_matches_rfc7636() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn jwt_signature_verifies_and_key_roundtrips() {
        let key = EsKey::generate();
        let restored = EsKey::from_hex(&key.to_hex()).unwrap();
        assert_eq!(key.thumbprint(), restored.thumbprint());

        let jwt = key.sign_jwt(&json!({"alg": "ES256", "typ": "dpop+jwt"}), &json!({"htm": "POST"}));
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let sig = Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2]).unwrap()).unwrap();
        let jwk = key.public_jwk();
        let mut sec1 = vec![4u8];
        sec1.extend(URL_SAFE_NO_PAD.decode(jwk["x"].as_str().unwrap()).unwrap());
        sec1.extend(URL_SAFE_NO_PAD.decode(jwk["y"].as_str().unwrap()).unwrap());
        let vk = VerifyingKey::from_sec1_bytes(&sec1).unwrap();
        vk.verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &sig).unwrap();
        let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims["htm"], "POST");
    }

    #[test]
    fn random_strings_are_unique() {
        assert_ne!(random_string(32), random_string(32));
        assert_eq!(random_string(32).len(), 43);
    }
}
