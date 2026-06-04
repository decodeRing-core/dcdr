use async_trait::async_trait;
use decodering_core::auth::method::{AuthMethod, Capabilities};
use decodering_core::auth::types::AuthError;
use decodering_core::auth::types::AuthRequest;
use decodering_core::auth::types::AuthResponse;
use decodering_core::auth::types::EnrollRequest;
use decodering_core::auth::types::EnrollResponse;
use decodering_core::crypto::sha256_hex;
use decodering_core::domain::PrincipalStatus;
use rand::distr::{Alphanumeric, SampleString};

const KEY_PREFIX: &str = "pk_";
const KEY_RANDOM_LEN: usize = 32;

pub struct ApiKeyMethod;

impl ApiKeyMethod {
    pub fn new() -> Self {
        Self
    }

    fn hash_key(raw: &str) -> String {
        sha256_hex(raw.as_bytes())
    }
}

impl Default for ApiKeyMethod {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthMethod for ApiKeyMethod {
    fn kind(&self) -> String {
        "apiKey".to_owned()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            kind: self.kind(),
            requires_activation: false,
            requires_challenge: false,
            requires_resolve: false,
        }
    }

    async fn enroll(&self, _req: EnrollRequest) -> Result<EnrollResponse, AuthError> {
        let random = Alphanumeric.sample_string(&mut rand::rng(), KEY_RANDOM_LEN);
        let raw_key = format!("{KEY_PREFIX}{random}");

        let lookup_key = Self::hash_key(&raw_key);

        Ok(EnrollResponse {
            lookup_key,
            secret_material: serde_json::json!({}),
            status: PrincipalStatus::Active,
            client_payload: Some(serde_json::json!({ "key": raw_key })),
        })
    }

    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse, AuthError> {
        let key = req
            .proof
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::InvalidProof("missing 'key' field".to_owned()))?;

        if !key.starts_with(KEY_PREFIX) {
            return Err(AuthError::InvalidProof("malformed key".to_owned()));
        }

        let lookup_key = Self::hash_key(key);

        Ok(AuthResponse {
            lookup_key,
            audit_metadata: serde_json::json!({
                "key_prefix": key.chars().take(KEY_PREFIX.len() + 8).collect::<String>(),
            }),
        })
    }
}
