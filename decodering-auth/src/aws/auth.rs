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
