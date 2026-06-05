use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use decodering_core::time::now_ts;
use rustls_pki_types::{CertificateDer, TrustAnchor, UnixTime};
use webpki::EndEntityCert;
use webpki::KeyUsage;
use webpki::anchor_from_trusted_cert;
use x509_parser::pem::parse_x509_pem;
use x509_parser::prelude::*;

#[derive(Debug)]
pub enum TrustStoreError {
    Io(io::Error),
    NoCertsFound,
    InvalidCert { path: PathBuf, reason: String },
}

impl From<io::Error> for TrustStoreError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

#[derive(Debug)]
pub enum EkCertError {
    InvalidPem,
    InvalidCertEncoding,
    PubkeyMismatch,
    NotYetValid,
    Expired,
    UntrustedIssuer,
    SignatureInvalid,
    NoTrustAnchorsLoaded,
}

/// Trust store for TPM EK certificate verification.
///
/// Holds DER-encoded certificates loaded from a directory. Both root and
/// intermediate manufacturer CAs go in the same directory and are treated
/// as trust anchors — so any chain that ends at one of them verifies.
pub struct TpmTrustStore {
    cert_der: Vec<CertificateDer<'static>>,
}

impl TpmTrustStore {
    pub fn from_directory(dir: impl AsRef<Path>) -> Result<Self, TrustStoreError> {
        let dir = dir.as_ref();
        let mut cert_der = Vec::new();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_ascii_lowercase);

            let bytes = fs::read(&path)?;

            let der = match ext.as_deref() {
                Some("der") => bytes,
                Some("pem" | "crt" | "cer") => parse_x509_pem(&bytes)
                    .map(|(_, pem)| pem.contents)
                    .map_err(|e| TrustStoreError::InvalidCert {
                        path: path.clone(),
                        reason: format!("PEM parse: {e}"),
                    })?,
                _ => continue,
            };

            // Sanity-check parse so bad files fail fast at startup.
            X509Certificate::from_der(&der).map_err(|e| TrustStoreError::InvalidCert {
                path: path.clone(),
                reason: format!("X509 parse: {e}"),
            })?;

            cert_der.push(der);
        }

        if cert_der.is_empty() {
            return Err(TrustStoreError::NoCertsFound);
        }

        let cert_der = cert_der.into_iter().map(CertificateDer::from).collect();

        Ok(Self { cert_der })
    }

    pub fn is_empty(&self) -> bool {
        self.cert_der.is_empty()
    }

    pub fn len(&self) -> usize {
        self.cert_der.len()
    }

    fn trust_anchors(&self) -> Vec<TrustAnchor<'_>> {
        self.cert_der
            .iter()
            .filter_map(|cd| anchor_from_trusted_cert(cd).ok())
            .collect()
    }
}

/// Verify an EK certificate.
pub fn verify_ek_cert_chain(
    ek_cert_pem: &str,
    expected_ek_pubkey_pem: &str,
    trust_store: &TpmTrustStore,
) -> Result<(), EkCertError> {
    // Parse EK cert PEM → DER
    let (_, ek_pem) =
        parse_x509_pem(ek_cert_pem.as_bytes()).map_err(|_| EkCertError::InvalidPem)?;
    let ek_cert_der_bytes = ek_pem.contents;

    let (_, parsed_cert) = X509Certificate::from_der(&ek_cert_der_bytes)
        .map_err(|_| EkCertError::InvalidCertEncoding)?;

    // Pubkey match
    let (_, expected_pem) =
        parse_x509_pem(expected_ek_pubkey_pem.as_bytes()).map_err(|_| EkCertError::InvalidPem)?;
    if parsed_cert.public_key().raw != expected_pem.contents.as_slice() {
        return Err(EkCertError::PubkeyMismatch);
    }

    // Validity window
    let now_secs = now_ts();
    if now_secs < parsed_cert.validity().not_before.timestamp() {
        return Err(EkCertError::NotYetValid);
    }
    if now_secs > parsed_cert.validity().not_after.timestamp() {
        return Err(EkCertError::Expired);
    }

    //  Chain verification
    let anchors = trust_store.trust_anchors();
    if anchors.is_empty() {
        return Err(EkCertError::NoTrustAnchorsLoaded);
    }

    let cert_der = CertificateDer::from(ek_cert_der_bytes.as_slice());
    let end_entity =
        EndEntityCert::try_from(&cert_der).map_err(|_| EkCertError::InvalidCertEncoding)?;

    let supported_sig_algs: &[&dyn rustls_pki_types::SignatureVerificationAlgorithm] = &[
        webpki::ring::RSA_PKCS1_2048_8192_SHA256,
        webpki::ring::RSA_PKCS1_2048_8192_SHA384,
        webpki::ring::RSA_PKCS1_2048_8192_SHA512,
        webpki::ring::ECDSA_P256_SHA256,
        webpki::ring::ECDSA_P384_SHA384,
    ];

    #[allow(clippy::expect_used)]
    let webpki_now = UnixTime::since_unix_epoch(Duration::from_secs(
        u64::try_from(now_secs).expect("system clock before UNIX_EPOCH"),
    ));

    end_entity
        .verify_for_usage(
            supported_sig_algs,
            &anchors,
            &[], // no caller-supplied intermediates
            webpki_now,
            KeyUsage::client_auth(),
            None, // no revocation checking
            None, // no path verifier
        )
        .map_err(|e| match e {
            webpki::Error::CertNotValidYet { .. } => EkCertError::NotYetValid,
            webpki::Error::CertExpired { .. } => EkCertError::Expired,
            webpki::Error::InvalidSignatureForPublicKey => EkCertError::SignatureInvalid,
            _ => EkCertError::UntrustedIssuer,
        })?;

    Ok(())
}
