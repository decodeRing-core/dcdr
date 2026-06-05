use async_trait::async_trait;

use decodering_core::auth::method::{AuthMethod, Capabilities};
use decodering_core::auth::types::AuthError;
use decodering_core::auth::types::AuthRequest;
use decodering_core::auth::types::AuthResponse;
use decodering_core::auth::types::EnrollRequest;
use decodering_core::auth::types::EnrollResponse;
use decodering_core::domain::PrincipalStatus;

use serde::Deserialize;
use std::collections::HashMap;

use crate::aws::utils::call_sts_get_caller_identity;
use crate::aws::utils::normalize_arn;
use crate::aws::utils::parse_iam_arn;
use crate::aws::utils::validate_sts_request;

pub struct AwsMethod;

impl AwsMethod {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AwsMethod {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct AwsEnrollData {
    role_arn: String,
}

#[derive(Deserialize)]
struct AwsAuthProof {
    method: String,
    url: String,
    body: String,
    headers: HashMap<String, String>,
}

#[async_trait]
impl AuthMethod for AwsMethod {
    fn kind(&self) -> String {
        "awsIdentity".to_owned()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            kind: self.kind(),
            requires_activation: false,
            requires_challenge: false,
            requires_resolve: false,
        }
    }

    async fn enroll(&self, req: EnrollRequest) -> Result<EnrollResponse, AuthError> {
        let data: AwsEnrollData = serde_json::from_value(req.data)
            .map_err(|e| AuthError::InvalidEnrollmentData(e.to_string()))?;

        let normalized = normalize_arn(&data.role_arn)
            .ok_or_else(|| AuthError::InvalidEnrollmentData("invalid role ARN".to_owned()))?;

        let parts = parse_iam_arn(&normalized)
            .ok_or_else(|| AuthError::InvalidEnrollmentData("unparseable role ARN".to_owned()))?;

        Ok(EnrollResponse {
            lookup_key: normalized,
            secret_material: serde_json::json!({
                "account_id": parts.account_id,
                "role_name":  parts.name,
            }),
            status: PrincipalStatus::Active,
            client_payload: None,
        })
    }

    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse, AuthError> {
        let proof: AwsAuthProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        validate_sts_request(&proof.method, &proof.url, &proof.body, &proof.headers)
            .map_err(|_| AuthError::InvalidProof("STS request validation failed".to_owned()))?;

        let identity = call_sts_get_caller_identity(&proof.url, proof.body, proof.headers)
            .await
            .map_err(|e| AuthError::VerificationFailed(format!("STS: {e}")))?;

        let normalized = normalize_arn(&identity.arn)
            .ok_or_else(|| AuthError::VerificationFailed("unparseable STS ARN".to_owned()))?;

        Ok(AuthResponse {
            lookup_key: normalized.clone(),
            audit_metadata: serde_json::json!({
                "arn": normalized,
                "account": identity.account,
            }),
        })
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enroll_req(data: serde_json::Value) -> EnrollRequest {
        EnrollRequest {
            principal_id: "p1".to_owned(),
            data,
            now: 0,
            config: json!({}),
        }
    }

    fn auth_req(proof: serde_json::Value) -> AuthRequest {
        AuthRequest {
            proof,
            challenge_state: None,
            credential_material: None,
            now: 0,
            config: json!({}),
        }
    }

    // ---- metadata ----

    #[test]
    fn kind_is_aws_identity() {
        assert_eq!(AwsMethod::new().kind(), "awsIdentity");
    }

    #[test]
    fn capabilities_are_self_contained() {
        let caps = AwsMethod::new().capabilities();
        assert_eq!(caps.kind, "awsIdentity");
        assert!(!caps.requires_activation);
        assert!(!caps.requires_challenge);
        assert!(!caps.requires_resolve);
    }

    #[test]
    fn default_matches_new() {
        assert_eq!(AwsMethod.kind(), AwsMethod::new().kind());
    }

    // ---- enroll ----

    #[tokio::test]
    async fn enroll_role_arn_succeeds() {
        let resp = AwsMethod::new()
            .enroll(enroll_req(json!({
                "role_arn": "arn:aws:iam::123456789012:role/MyRole"
            })))
            .await
            .unwrap();

        assert_eq!(resp.lookup_key, "arn:aws:iam::123456789012:role/MyRole");
        assert!(matches!(resp.status, PrincipalStatus::Active));
        assert!(resp.client_payload.is_none());
        assert_eq!(
            resp.secret_material.get("account_id").unwrap(),
            "123456789012"
        );
        assert_eq!(resp.secret_material.get("role_name").unwrap(), "MyRole");
    }

    #[tokio::test]
    async fn enroll_user_arn_succeeds() {
        let resp = AwsMethod::new()
            .enroll(enroll_req(json!({
                "role_arn": "arn:aws:iam::123456789012:user/Alice"
            })))
            .await
            .unwrap();

        assert_eq!(resp.lookup_key, "arn:aws:iam::123456789012:user/Alice");
        assert_eq!(resp.secret_material.get("role_name").unwrap(), "Alice");
    }

    #[tokio::test]
    async fn enroll_normalizes_assumed_role() {
        let resp = AwsMethod::new()
            .enroll(enroll_req(json!({
                "role_arn": "arn:aws:sts::123456789012:assumed-role/MyRole/sess"
            })))
            .await
            .unwrap();

        assert_eq!(resp.lookup_key, "arn:aws:iam::123456789012:role/MyRole");
        assert_eq!(resp.secret_material.get("role_name").unwrap(), "MyRole");
    }

    #[tokio::test]
    async fn enroll_rejects_missing_role_arn_field() {
        let err = AwsMethod::new()
            .enroll(enroll_req(json!({})))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    #[tokio::test]
    async fn enroll_rejects_wrong_type() {
        let err = AwsMethod::new()
            .enroll(enroll_req(json!({ "role_arn": 123 })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    #[tokio::test]
    async fn enroll_rejects_unnormalizable_arn() {
        let err = AwsMethod::new()
            .enroll(enroll_req(json!({ "role_arn": "not-an-arn" })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    #[tokio::test]
    async fn enroll_rejects_federated_user_arn() {
        let err = AwsMethod::new()
            .enroll(enroll_req(json!({
                "role_arn": "arn:aws:sts::123456789012:federated-user/Alice"
            })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    // ---- authenticate (non-network branches) ----

    #[tokio::test]
    async fn authenticate_rejects_malformed_proof() {
        let err = AwsMethod::new()
            .authenticate(auth_req(json!({ "method": "POST" })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_non_object_proof() {
        let err = AwsMethod::new()
            .authenticate(auth_req(json!("string")))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_invalid_sts_url() {
        let err = AwsMethod::new()
            .authenticate(auth_req(json!({
                "method": "POST",
                "url": "https://evil.example.com/",
                "body": "Action=GetCallerIdentity",
                "headers": { "Authorization": "x" }
            })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_wrong_action() {
        let err = AwsMethod::new()
            .authenticate(auth_req(json!({
                "method": "POST",
                "url": "https://sts.amazonaws.com/",
                "body": "Action=AssumeRole",
                "headers": { "Authorization": "x" }
            })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_missing_authorization_header() {
        let err = AwsMethod::new()
            .authenticate(auth_req(json!({
                "method": "POST",
                "url": "https://sts.amazonaws.com/",
                "body": "Action=GetCallerIdentity",
                "headers": {}
            })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }
}
