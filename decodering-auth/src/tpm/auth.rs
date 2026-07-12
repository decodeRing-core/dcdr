use async_trait::async_trait;

use constant_time_eq::constant_time_eq;
use decodering_core::auth::method::{AuthMethod, Capabilities};
use decodering_core::auth::types::ActivateRequest;
use decodering_core::auth::types::ActivateResponse;
use decodering_core::auth::types::AuthError;
use decodering_core::auth::types::AuthRequest;
use decodering_core::auth::types::AuthResponse;
use decodering_core::auth::types::ChallengeRequest;
use decodering_core::auth::types::ChallengeResponse;
use decodering_core::auth::types::EnrollRequest;
use decodering_core::auth::types::EnrollResponse;
use decodering_core::auth::types::ResolveRequest;
use decodering_core::crypto::base64_decode;
use decodering_core::crypto::base64_encode;
use decodering_core::crypto::encode_hex;
use decodering_core::crypto::pem_to_der;
use decodering_core::crypto::sha256_hex;
use decodering_core::domain::PrincipalStatus;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::tpm::cert::TpmTrustStore;
use crate::tpm::cert::verify_ek_cert_chain;
use crate::tpm::utils::AkPublic;
use crate::tpm::utils::{
    make_credential_rsa, parse_tpms_attest, verify_pcrs, verify_quote_signature,
};

#[derive(Deserialize)]
struct TpmEnrollData {
    ek_pubkey_pem: String,
    #[serde(default)]
    ek_cert_pem: Option<String>,
    ak_public_tpm2b_b64: String,
    #[serde(default)]
    expected_pcrs: Option<HashMap<u8, String>>,
    #[serde(default)]
    require_ek_cert: bool,
}

#[derive(Serialize, Deserialize)]
struct TpmCredentialMaterial {
    ek_pubkey_pem: String,
    ek_cert_pem: Option<String>,
    require_ek_cert: bool,
    expected_pcrs: Option<HashMap<u8, String>>,
    ak_pubkey_pem: String,
    ak_name_hex: String,
    activation_secret_hash: String,
}

#[derive(Deserialize)]
struct TpmAuthProof {
    ek_pubkey_pem: String,
    ak_pubkey_pem: String,
    quote: String,
    signature: String,
    #[serde(default)]
    pcrs: Option<HashMap<u8, String>>,
}

#[derive(Deserialize)]
struct TpmActivateProof {
    recovered_secret: String, // base64
}

pub struct TpmMethod;

impl TpmMethod {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TpmMethod {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthMethod for TpmMethod {
    fn kind(&self) -> String {
        "trustedPlatformModule".to_owned()
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            kind: self.kind(),
            requires_activation: true,
            requires_challenge: true,
            requires_resolve: true,
        }
    }

    async fn enroll(&self, req: EnrollRequest) -> Result<EnrollResponse, AuthError> {
        let data: TpmEnrollData = serde_json::from_value(req.data)
            .map_err(|e| AuthError::InvalidEnrollmentData(e.to_string()))?;

        // lookup_key = sha256(EK DER)
        let ek_der = pem_to_der(&data.ek_pubkey_pem)
            .map_err(|e| AuthError::InvalidEnrollmentData(format!("bad EK: {e}")))?;
        let lookup_key = sha256_hex(&ek_der);

        // Optional EK cert chain check (deferred path; matches your require_ek_cert)
        if data.require_ek_cert {
            let cert = data.ek_cert_pem.as_ref().ok_or_else(|| {
                AuthError::InvalidEnrollmentData("require_ek_cert set but no cert".to_owned())
            })?;
            let tpm_trust_dir = req
                .config
                .get("tpm_trust_dir")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();
            let trust_store = TpmTrustStore::from_directory(&tpm_trust_dir)
                .map_err(|_| AuthError::Internal("trust store load failed".to_owned()))?;
            verify_ek_cert_chain(cert, &data.ek_pubkey_pem, &trust_store)
                .map_err(|e| AuthError::VerificationFailed(format!("EK cert: {e:#?}")))?;
        }

        // Parse AK from the TPM2B, derive its name
        let ak_pub_2b = base64_decode(&data.ak_public_tpm2b_b64)
            .ok_or_else(|| AuthError::InvalidEnrollmentData("AK b64".to_owned()))?;
        let ak = AkPublic::parse(&ak_pub_2b)
            .map_err(|e| AuthError::InvalidEnrollmentData(format!("parse AK: {e}")))?;

        // MakeCredential: random secret sealed to EK, bound to AK name
        let mut secret = [0u8; 32];
        rand::rng().fill_bytes(&mut secret);
        let challenge = make_credential_rsa(&data.ek_pubkey_pem, &ak.name, &secret)
            .map_err(|e| AuthError::Internal(format!("make_credential: {e}")))?;

        let material = TpmCredentialMaterial {
            ek_pubkey_pem: data.ek_pubkey_pem,
            ek_cert_pem: data.ek_cert_pem,
            require_ek_cert: data.require_ek_cert,
            expected_pcrs: data.expected_pcrs,
            ak_pubkey_pem: ak.pubkey_pem,
            ak_name_hex: encode_hex(&ak.name),
            activation_secret_hash: sha256_hex(&secret),
        };

        Ok(EnrollResponse {
            lookup_key,
            secret_material: serde_json::to_value(material)
                .map_err(|e| AuthError::Internal(e.to_string()))?,
            status: PrincipalStatus::Pending,
            client_payload: Some(serde_json::json!({
                "credential_blob": base64_encode(challenge.credential_blob),
                "secret": base64_encode(challenge.secret),
            })),
        })
    }

    async fn activate(&self, req: ActivateRequest) -> Result<ActivateResponse, AuthError> {
        // Host supplies the credential's secret_material here.
        let material: TpmCredentialMaterial = serde_json::from_value(req.credential_material)
            .map_err(|e| AuthError::Internal(format!("bad material: {e}")))?;

        let proof: TpmActivateProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        let recovered = base64_decode(&proof.recovered_secret)
            .ok_or_else(|| AuthError::InvalidProof("recovered_secret b64".to_owned()))?;

        if !constant_time_eq(
            sha256_hex(&recovered).as_bytes(),
            material.activation_secret_hash.as_bytes(),
        ) {
            return Err(AuthError::ActivationFailed("secret mismatch".to_owned()));
        }
        Ok(ActivateResponse { activated: true })
    }

    async fn challenge(&self, req: ChallengeRequest) -> Result<ChallengeResponse, AuthError> {
        let nonce = req.entropy;
        let nonce_hex = encode_hex(&nonce);

        Ok(ChallengeResponse {
            client_payload: serde_json::json!({ "nonce": nonce_hex }),
            challenge_state: nonce,
        })
    }

    async fn resolve(&self, req: ResolveRequest) -> Result<String, AuthError> {
        let proof: TpmAuthProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        let ek_der = pem_to_der(&proof.ek_pubkey_pem)
            .map_err(|e| AuthError::InvalidProof(format!("bad EK pubkey: {e}")))?;
        Ok(sha256_hex(&ek_der))
    }

    async fn authenticate(&self, req: AuthRequest) -> Result<AuthResponse, AuthError> {
        let proof: TpmAuthProof = serde_json::from_value(req.proof)
            .map_err(|e| AuthError::InvalidProof(e.to_string()))?;

        let material_val = req
            .credential_material
            .ok_or_else(|| AuthError::Internal("missing credential_material".to_owned()))?;
        let material: TpmCredentialMaterial = serde_json::from_value(material_val)
            .map_err(|e| AuthError::Internal(format!("bad credential_material: {e}")))?;

        if proof.ek_pubkey_pem.trim() != material.ek_pubkey_pem.trim() {
            return Err(AuthError::VerificationFailed(
                "presented EK does not match enrolled EK".to_owned(),
            ));
        }

        let challenge_nonce = req.challenge_state.ok_or_else(|| {
            AuthError::Internal("missing challenge_state for TPM auth".to_owned())
        })?;

        let quote_bytes = base64_decode(&proof.quote)
            .ok_or_else(|| AuthError::InvalidProof("quote not base64".to_owned()))?;
        let sig_bytes = base64_decode(&proof.signature)
            .ok_or_else(|| AuthError::InvalidProof("signature not base64".to_owned()))?;

        let attest = parse_tpms_attest(&quote_bytes)
            .map_err(|e| AuthError::VerificationFailed(format!("bad quote: {e}")))?;

        if attest.extra_data != challenge_nonce {
            return Err(AuthError::VerificationFailed(
                "quote nonce does not match challenge".to_owned(),
            ));
        }

        verify_quote_signature(&quote_bytes, &sig_bytes, &proof.ak_pubkey_pem)
            .map_err(|e| AuthError::VerificationFailed(format!("signature: {e}")))?;

        if let Some(expected) = &material.expected_pcrs {
            let pcrs = proof.pcrs.as_ref().ok_or_else(|| {
                AuthError::VerificationFailed("credential pins PCRs but none sent".to_owned())
            })?;
            verify_pcrs(pcrs, &attest.pcr_digest, expected, &attest.pcr_selections)
                .map_err(|e| AuthError::VerificationFailed(format!("pcr: {e}")))?;
        }

        let ek_der = pem_to_der(&proof.ek_pubkey_pem)
            .map_err(|e| AuthError::InvalidProof(format!("bad EK pubkey: {e}")))?;

        let ek_hash = sha256_hex(&ek_der);
        Ok(AuthResponse {
            lookup_key: ek_hash.clone(),
            audit_metadata: serde_json::json!({
                "ek_sha256_prefix": ek_hash.chars().take(16).collect::<String>(),
            }),
        })
    }
}

#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
#[allow(clippy::cast_possible_truncation)]
#[cfg(test)]
mod tests {
    use super::*;
    use decodering_core::crypto::sha256_hex;
    use rsa::pkcs1v15::SigningKey as Pkcs1v15SigningKey;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::signature::{SignatureEncoding, Signer as _};
    use rsa::{RsaPrivateKey, RsaPublicKey, traits::PublicKeyParts};
    use serde_json::json;
    use sha2::{Digest, Sha256};

    const TPM_GENERATED_VALUE: u32 = 0xFF54_4347;
    const TPM_ST_ATTEST_QUOTE: u16 = 0x8018;
    const TPM_ALG_RSA: u16 = 0x0001;
    const TPM_ALG_NULL: u16 = 0x0010;
    const TPM_ALG_SHA256: u16 = 0x000B;
    const TPM_ALG_RSASSA: u16 = 0x0014;

    fn rsa_keypair() -> (RsaPrivateKey, String) {
        let priv_key = RsaPrivateKey::new(&mut rand::rng(), 2048).unwrap();
        let pem = RsaPublicKey::from(&priv_key)
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        (priv_key, pem)
    }

    /// Build a marshaled `TPM2B_PUBLIC` for an RSA key matchin`AkPublic::parse`se's
    /// expected layout. Modulus is taken from `pub_key`.
    fn build_ak_tpm2b(pub_key: &RsaPublicKey) -> Vec<u8> {
        let modulus = pub_key.n().to_be_bytes();
        let mut pa = Vec::new();
        pa.extend_from_slice(&TPM_ALG_RSA.to_be_bytes()); // type
        pa.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes()); // nameAlg
        pa.extend_from_slice(&0u32.to_be_bytes()); // objectAttributes
        pa.extend_from_slice(&0u16.to_be_bytes()); // authPolicy (TPM2B, empty)
        pa.extend_from_slice(&TPM_ALG_NULL.to_be_bytes()); // symmetric = NULL
        pa.extend_from_slice(&TPM_ALG_NULL.to_be_bytes()); // scheme = NULL
        pa.extend_from_slice(&2048u16.to_be_bytes()); // keyBits
        pa.extend_from_slice(&0u32.to_be_bytes()); // exponent = 0 => 65537
        pa.extend_from_slice(&(modulus.len() as u16).to_be_bytes());
        pa.extend_from_slice(&modulus); // unique (modulus)

        let mut out = Vec::new();
        out.extend_from_slice(&(pa.len() as u16).to_be_bytes());
        out.extend_from_slice(&pa);
        out
    }

    /// Compute the AK Name the same way `AkPublic::parse` does:
    /// UINT16(nameAlg) || SHA256(publicArea).
    fn ak_name_from_tpm2b(tpm2b: &[u8]) -> Vec<u8> {
        let pa_len = u16::from_be_bytes([tpm2b[0], tpm2b[1]]) as usize;
        let public_area = &tpm2b[2..2 + pa_len];
        let mut name = Vec::new();
        name.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        name.extend_from_slice(&Sha256::digest(public_area));
        name
    }

    fn build_tpms_attest(extra_data: &[u8], pcr_digest: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&TPM_GENERATED_VALUE.to_be_bytes());
        buf.extend_from_slice(&TPM_ST_ATTEST_QUOTE.to_be_bytes());
        buf.extend_from_slice(&0u16.to_be_bytes()); // qualifiedSigner (empty)
        buf.extend_from_slice(&(extra_data.len() as u16).to_be_bytes());
        buf.extend_from_slice(extra_data);
        buf.extend_from_slice(&[0u8; 17]); // clockInfo
        buf.extend_from_slice(&[0u8; 8]); // firmwareVersion
        // One PCR selection: SHA256, PCR 0
        buf.extend_from_slice(&1u32.to_be_bytes()); // count
        buf.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        buf.push(1); // sizeOfSelect
        buf.push(0b0000_0001); // PCR 0
        buf.extend_from_slice(&(pcr_digest.len() as u16).to_be_bytes());
        buf.extend_from_slice(pcr_digest);
        buf
    }

    fn build_rsa_tpmt_sig(sig_bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&TPM_ALG_RSASSA.to_be_bytes());
        out.extend_from_slice(&TPM_ALG_SHA256.to_be_bytes());
        out.extend_from_slice(&(sig_bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(sig_bytes);
        out
    }

    fn enroll_req(data: serde_json::Value, config: serde_json::Value) -> EnrollRequest {
        EnrollRequest {
            principal_id: "p1".to_owned(),
            data,
            now: 0,
            config,
        }
    }
    fn auth_req(
        proof: serde_json::Value,
        cs: Option<Vec<u8>>,
        cm: Option<serde_json::Value>,
    ) -> AuthRequest {
        AuthRequest {
            proof,
            challenge_state: cs,
            credential_material: cm,
            now: 0,
            config: json!({}),
        }
    }
    fn activate_req(cm: serde_json::Value, proof: serde_json::Value) -> ActivateRequest {
        ActivateRequest {
            principal_id: "p1".to_owned(),
            credential_id: "c1".to_owned(),
            credential_material: cm,
            proof,
            now: 0,
            config: json!({}),
        }
    }

    #[test]
    fn kind_and_capabilities() {
        let m = TpmMethod::new();
        assert_eq!(m.kind(), "trustedPlatformModule");
        let c = m.capabilities();
        assert!(c.requires_activation && c.requires_challenge && c.requires_resolve);
    }

    #[test]
    fn default_constructs() {
        let _ = <TpmMethod as Default>::default();
    }

    #[tokio::test]
    async fn challenge_echoes_entropy() {
        let resp = TpmMethod::new()
            .challenge(ChallengeRequest {
                hint: None,
                entropy: vec![0xab, 0xcd],
                now: 0,
                config: json!({}),
            })
            .await
            .unwrap();
        assert_eq!(resp.challenge_state, vec![0xab, 0xcd]);
        assert_eq!(resp.client_payload["nonce"], "abcd");
    }

    #[tokio::test]
    async fn resolve_returns_ek_hash() {
        let (_, ek_pem) = rsa_keypair();
        let ek_der = pem_to_der(&ek_pem).unwrap();
        let resp = TpmMethod::new()
            .resolve(ResolveRequest {
                proof: json!({
                    "ek_pubkey_pem": ek_pem, "ak_pubkey_pem": "x",
                    "quote": "", "signature": ""
                }),
                config: json!({}),
            })
            .await
            .unwrap();
        assert_eq!(resp, sha256_hex(&ek_der));
    }

    #[tokio::test]
    async fn resolve_rejects_bad_proof() {
        let err = TpmMethod::new()
            .resolve(ResolveRequest {
                proof: json!({}),
                config: json!({}),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn enroll_succeeds_end_to_end() {
        let (_, ek_pem) = rsa_keypair();
        let (ak_priv, _) = rsa_keypair();
        let ak_pub = RsaPublicKey::from(&ak_priv);
        let ak_2b = build_ak_tpm2b(&ak_pub);
        let ak_2b_b64 = base64_encode(ak_2b.clone());

        let resp = TpmMethod::new()
            .enroll(enroll_req(
                json!({
                    "ek_pubkey_pem": ek_pem,
                    "ak_public_tpm2b_b64": ak_2b_b64,
                }),
                json!({}),
            ))
            .await
            .unwrap();

        assert!(matches!(resp.status, PrincipalStatus::Pending));
        let ek_der = pem_to_der(&ek_pem).unwrap();
        assert_eq!(resp.lookup_key, sha256_hex(&ek_der));

        let payload = resp.client_payload.unwrap();
        assert!(payload.get("credential_blob").is_some());
        assert!(payload.get("secret").is_some());

        // Material round-trips and carries the derived AK name.
        let expected_name = ak_name_from_tpm2b(&ak_2b);
        assert_eq!(
            resp.secret_material["ak_name_hex"],
            encode_hex(&expected_name)
        );
        assert_eq!(resp.secret_material["require_ek_cert"], false);
    }

    #[tokio::test]
    async fn enroll_rejects_missing_fields() {
        let err = TpmMethod::new()
            .enroll(enroll_req(json!({}), json!({})))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    #[tokio::test]
    async fn enroll_rejects_bad_ek() {
        let err = TpmMethod::new()
            .enroll(enroll_req(
                json!({ "ek_pubkey_pem": "nope", "ak_public_tpm2b_b64": "AAAA" }),
                json!({}),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    #[tokio::test]
    async fn enroll_rejects_bad_ak_b64() {
        let (_, ek_pem) = rsa_keypair();
        let err = TpmMethod::new()
            .enroll(enroll_req(
                json!({ "ek_pubkey_pem": ek_pem, "ak_public_tpm2b_b64": "!!!" }),
                json!({}),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    #[tokio::test]
    async fn enroll_require_ek_cert_without_cert_fails() {
        let (_, ek_pem) = rsa_keypair();
        let (ak_priv, _) = rsa_keypair();
        let ak_2b_b64 = base64_encode(build_ak_tpm2b(&RsaPublicKey::from(&ak_priv)));
        let err = TpmMethod::new()
            .enroll(enroll_req(
                json!({
                    "ek_pubkey_pem": ek_pem,
                    "ak_public_tpm2b_b64": ak_2b_b64,
                    "require_ek_cert": true
                }),
                json!({}),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidEnrollmentData(_)));
    }

    fn material_json(ek_pem: &str, ak_pem: &str, secret_hash: &str) -> serde_json::Value {
        json!({
            "ek_pubkey_pem": ek_pem, "ek_cert_pem": null, "require_ek_cert": false,
            "expected_pcrs": null, "ak_pubkey_pem": ak_pem, "ak_name_hex": "00",
            "activation_secret_hash": secret_hash
        })
    }

    #[tokio::test]
    async fn activate_accepts_correct_secret() {
        let secret = b"the-secret".to_vec();
        let mat = material_json("e", "a", &sha256_hex(&secret));
        let resp = TpmMethod::new()
            .activate(activate_req(
                mat,
                json!({ "recovered_secret": base64_encode(secret) }),
            ))
            .await
            .unwrap();
        assert!(resp.activated);
    }

    #[tokio::test]
    async fn activate_rejects_wrong_secret() {
        let mat = material_json("e", "a", &sha256_hex(b"real"));
        let err = TpmMethod::new()
            .activate(activate_req(
                mat,
                json!({ "recovered_secret": base64_encode(b"wrong".to_vec()) }),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::ActivationFailed(_)));
    }

    #[tokio::test]
    async fn activate_rejects_bad_material() {
        let err = TpmMethod::new()
            .activate(activate_req(
                json!({}),
                json!({ "recovered_secret": "AAAA" }),
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::Internal(_)));
    }

    #[tokio::test]
    async fn activate_rejects_non_b64_secret() {
        let mat = material_json("e", "a", &sha256_hex(b"x"));
        let err = TpmMethod::new()
            .activate(activate_req(mat, json!({ "recovered_secret": "!!!" })))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    /// Build a complete, valid auth request: EK matches material, AK signs a
    /// quote whose extraData == nonce.
    fn valid_auth_setup(
        nonce: &[u8],
        pcr_digest: &[u8],
        expected_pcrs: Option<&serde_json::Value>,
        pcrs_in_proof: Option<serde_json::Value>,
    ) -> (serde_json::Value, Vec<u8>, serde_json::Value) {
        let (_, ek_pem) = rsa_keypair();
        let (ak_priv, ak_pem) = rsa_keypair();

        let quote = build_tpms_attest(nonce, pcr_digest);
        let signing_key = Pkcs1v15SigningKey::<Sha256>::new(ak_priv);
        let sig = signing_key.sign(&quote).to_bytes();
        let tpmt_sig = build_rsa_tpmt_sig(&sig);

        let mut proof = json!({
            "ek_pubkey_pem": ek_pem,
            "ak_pubkey_pem": ak_pem,
            "quote": base64_encode(quote),
            "signature": base64_encode(tpmt_sig),
        });
        if let Some(p) = pcrs_in_proof {
            proof["pcrs"] = p;
        }

        let material = json!({
            "ek_pubkey_pem": ek_pem, "ek_cert_pem": null, "require_ek_cert": false,
            "expected_pcrs": expected_pcrs, "ak_pubkey_pem": ak_pem,
            "ak_name_hex": "00", "activation_secret_hash": sha256_hex(b"x")
        });
        (proof, nonce.to_vec(), material)
    }

    #[tokio::test]
    async fn authenticate_succeeds_without_pcrs() {
        let nonce = vec![1, 2, 3, 4];
        let (proof, cs, mat) = valid_auth_setup(&nonce, &[0u8; 32], None, None);
        let ek_pem = proof["ek_pubkey_pem"].as_str().unwrap().to_owned();

        let resp = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap();

        let ek_der = pem_to_der(&ek_pem).unwrap();
        assert_eq!(resp.lookup_key, sha256_hex(&ek_der));
        assert_eq!(
            resp.audit_metadata["ek_sha256_prefix"],
            sha256_hex(&ek_der).chars().take(16).collect::<String>()
        );
    }

    #[tokio::test]
    async fn authenticate_succeeds_with_pcr_policy() {
        // pcrDigest must equal SHA256(pcr0_value) for the single PCR-0 selection.
        let pcr0 = [0xAAu8; 32];
        let digest = Sha256::digest(pcr0);
        let pcrs = json!({ "0": "aa".repeat(32) });
        let expected = json!({ "0": "aa".repeat(32) });

        let nonce = vec![9, 9];
        let (proof, cs, mat) = valid_auth_setup(&nonce, &digest, Some(&expected), Some(pcrs));

        let resp = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap();
        assert!(!resp.lookup_key.is_empty());
    }

    #[tokio::test]
    async fn authenticate_rejects_pcr_policy_without_pcrs_sent() {
        let pcr0 = [0xAAu8; 32];
        let digest = Sha256::digest(pcr0);
        let expected = json!({ "0": "aa".repeat(32) });
        let nonce = vec![1];
        // No pcrs in proof.
        let (proof, cs, mat) = valid_auth_setup(&nonce, &digest, Some(&expected), None);

        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_nonce_mismatch() {
        let (proof, _cs, mat) = valid_auth_setup(&[1, 2, 3], &[0u8; 32], None, None);
        // Hand a different challenge_state than what's baked into the quote.
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(vec![9, 9, 9]), Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_bad_signature() {
        let nonce = vec![1, 2, 3, 4];
        let (mut proof, cs, mat) = valid_auth_setup(&nonce, &[0u8; 32], None, None);
        // Corrupt the signature (still valid base64, wrong bytes).
        proof["signature"] = json!(base64_encode(build_rsa_tpmt_sig(&[0xFFu8; 256])));
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_ek_mismatch() {
        let nonce = vec![1];
        let (mut proof, cs, mat) = valid_auth_setup(&nonce, &[0u8; 32], None, None);
        let (_, other_ek) = rsa_keypair();
        proof["ek_pubkey_pem"] = json!(other_ek);
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_missing_challenge_state() {
        let (proof, _cs, mat) = valid_auth_setup(&[1], &[0u8; 32], None, None);
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, None, Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::Internal(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_missing_material() {
        let (proof, cs, _mat) = valid_auth_setup(&[1], &[0u8; 32], None, None);
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), None))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::Internal(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_non_b64_quote() {
        let (mut proof, cs, mat) = valid_auth_setup(&[1], &[0u8; 32], None, None);
        proof["quote"] = json!("!!!");
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::InvalidProof(_)));
    }

    #[tokio::test]
    async fn authenticate_rejects_unparseable_quote() {
        let (mut proof, cs, mat) = valid_auth_setup(&[1], &[0u8; 32], None, None);
        proof["quote"] = json!(base64_encode(b"garbage".to_vec()));
        let err = TpmMethod::new()
            .authenticate(auth_req(proof, Some(cs), Some(mat)))
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::VerificationFailed(_)));
    }
}
