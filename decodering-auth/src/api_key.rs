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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    fn req(proof: serde_json::Value) -> AuthRequest {
        AuthRequest {
            proof,
            challenge_state: None,
            credential_material: None,
            now: 0,
            config: json!({}),
        }
    }

    fn enroll_req() -> EnrollRequest {
        EnrollRequest {
            principal_id: "principal-1".to_owned(),
            data: json!({}),
            now: 0,
            config: json!({}),
        }
    }

    #[test]
    fn kind_is_api_key() {
        assert_eq!(ApiKeyMethod.kind(), "apiKey");
    }

    #[test]
    fn capabilities_are_all_false() {
        let caps = ApiKeyMethod.capabilities();
        assert_eq!(caps.kind, "apiKey");
        assert!(!caps.requires_activation);
        assert!(!caps.requires_challenge);
        assert!(!caps.requires_resolve);
    }

    #[test]
    fn hash_key_is_deterministic() {
        let a = ApiKeyMethod::hash_key("pk_abc");
        let b = ApiKeyMethod::hash_key("pk_abc");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_key_differs_for_different_input() {
        assert_ne!(
            ApiKeyMethod::hash_key("pk_abc"),
            ApiKeyMethod::hash_key("pk_xyz")
        );
    }

    #[test]
    fn hash_key_matches_sha256_hex() {
        assert_eq!(ApiKeyMethod::hash_key("pk_abc"), sha256_hex(b"pk_abc"));
    }

    #[tokio::test]
    async fn enroll_produces_valid_key() {
        let m = ApiKeyMethod::new();
        let resp = m.enroll(enroll_req()).await.unwrap();

        assert_eq!(resp.status, PrincipalStatus::Active);
        assert_eq!(resp.secret_material, json!({}));

        let raw = resp
            .client_payload
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        assert!(raw.starts_with(KEY_PREFIX));
        assert_eq!(raw.len(), KEY_PREFIX.len() + KEY_RANDOM_LEN);

        // lookup_key is the hash of the raw key
        assert_eq!(resp.lookup_key, ApiKeyMethod::hash_key(&raw));
    }

    #[tokio::test]
    async fn enroll_generates_unique_keys() {
        let m = ApiKeyMethod::new();
        let a = m.enroll(enroll_req()).await.unwrap();
        let b = m.enroll(enroll_req()).await.unwrap();
        assert_ne!(a.lookup_key, b.lookup_key);
    }

    #[tokio::test]
    async fn authenticate_succeeds_with_valid_key() {
        let m = ApiKeyMethod::new();
        let raw = "pk_0123456789abcdef";
        let resp = m.authenticate(req(json!({ "key": raw }))).await.unwrap();

        assert_eq!(resp.lookup_key, ApiKeyMethod::hash_key(raw));
        assert_eq!(
            resp.audit_metadata
                .get("key_prefix")
                .unwrap()
                .as_str()
                .unwrap(),
            "pk_01234567" // KEY_PREFIX (3) + 8 chars
        );
    }

    #[tokio::test]
    async fn authenticate_round_trips_with_enroll() {
        let m = ApiKeyMethod::new();
        let enrolled = m.enroll(enroll_req()).await.unwrap();
        let raw = enrolled
            .client_payload
            .unwrap()
            .get("key")
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();

        let authed = m.authenticate(req(json!({ "key": raw }))).await.unwrap();
        assert_eq!(authed.lookup_key, enrolled.lookup_key);
    }

    #[tokio::test]
    async fn authenticate_fails_when_key_missing() {
        let m = ApiKeyMethod::new();
        let err = m.authenticate(req(json!({}))).await.unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_fails_when_key_not_string() {
        let m = ApiKeyMethod::new();
        let err = m
            .authenticate(req(json!({ "key": 123 })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_fails_on_malformed_prefix() {
        let m = ApiKeyMethod::new();
        let err = m
            .authenticate(req(json!({ "key": "xx_nope" })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_prefix_truncates_short_keys() {
        let m = ApiKeyMethod::new();
        let raw = "pk_ab"; // shorter than prefix+8
        let resp = m.authenticate(req(json!({ "key": raw }))).await.unwrap();
        assert_eq!(
            resp.audit_metadata
                .get("key_prefix")
                .unwrap()
                .as_str()
                .unwrap(),
            raw
        );
    }
}
