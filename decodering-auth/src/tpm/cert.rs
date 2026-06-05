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

#[allow(clippy::unwrap_used)]
#[allow(clippy::cast_possible_truncation)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use rcgen::{
        BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose,
        IsCa, Issuer, KeyPair, KeyUsagePurpose,
    };

    struct Ca {
        cert: rcgen::Certificate,
        issuer: Issuer<'static, KeyPair>,
    }

    fn dn(cn: &str) -> DistinguishedName {
        let mut d = DistinguishedName::new();
        d.push(DnType::CommonName, cn);
        d
    }

    fn make_ca() -> Ca {
        let mut params = CertificateParams::new(vec![]).unwrap();
        params.distinguished_name = dn("Test TPM CA");
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        let key = KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        let issuer = Issuer::new(params, key);
        Ca { cert, issuer }
    }

    /// Returns (`ek_cert_pem`, `ek_spki_pubkey_pem`). The leaf is signed by `ca`,
    /// has client-auth EKU (`verify_for_usage` requires `client_auth`), and a
    /// validity window controlled by the caller.
    fn make_ek_leaf(
        ca: &Ca,
        not_before_offset_days: i64,
        not_after_offset_days: i64,
    ) -> (String, String) {
        use rcgen::date_time_ymd;
        let mut params = CertificateParams::new(vec![]).unwrap();
        params.distinguished_name = dn("Test EK");
        params.is_ca = IsCa::NoCa;
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        // Crude window control via fixed dates relative to a base year.
        let base = 2025;
        params.not_before = date_time_ymd((base) + (not_before_offset_days / 365) as i32, 1, 1);
        params.not_after = date_time_ymd((base) + (not_after_offset_days / 365) as i32, 1, 1);
        let leaf_key = KeyPair::generate().unwrap();
        let leaf = params.signed_by(&leaf_key, &ca.issuer).unwrap();
        (leaf.pem(), leaf_key.public_key_pem())
    }

    fn trust_store_from_pems(pems: &[String], tag: &str) -> TpmTrustStore {
        let d = tmpdir(tag);
        for (i, pem) in pems.iter().enumerate() {
            fs::write(d.join(format!("ca{i}.pem")), pem).unwrap();
        }
        let store = TpmTrustStore::from_directory(&d).unwrap();
        let _ = fs::remove_dir_all(&d);
        store
    }

    #[test]
    fn verify_ek_cert_rejects_invalid_pem() {
        let ca = make_ca();
        let store = trust_store_from_pems(&[ca.cert.pem()], "vbadpem");
        let (_, ek_pub) = make_ek_leaf(&ca, -365, 3650);
        let r = verify_ek_cert_chain("not a pem", &ek_pub, &store);
        assert!(matches!(r, Err(EkCertError::InvalidPem)));
    }

    #[test]
    fn verify_ek_cert_rejects_invalid_expected_pubkey_pem() {
        let ca = make_ca();
        let store = trust_store_from_pems(&[ca.cert.pem()], "vbadexp");
        let (ek_cert, _) = make_ek_leaf(&ca, -365, 3650);
        let r = verify_ek_cert_chain(&ek_cert, "garbage", &store);
        assert!(matches!(r, Err(EkCertError::InvalidPem)));
    }

    #[test]
    fn verify_ek_cert_rejects_pubkey_mismatch() {
        let ca = make_ca();
        let store = trust_store_from_pems(&[ca.cert.pem()], "vmismatch");
        let (ek_cert, _) = make_ek_leaf(&ca, -365, 3650);
        // Expected pubkey from a *different* leaf.
        let (_, other_pub) = make_ek_leaf(&ca, -365, 3650);
        let r = verify_ek_cert_chain(&ek_cert, &other_pub, &store);
        assert!(matches!(r, Err(EkCertError::PubkeyMismatch)));
    }

    #[test]
    fn verify_ek_cert_rejects_expired() {
        let ca = make_ca();
        let store = trust_store_from_pems(&[ca.cert.pem()], "vexpired");
        // not_after well in the past.
        let (ek_cert, ek_pub) = make_ek_leaf(&ca, -3650, -1825);
        let r = verify_ek_cert_chain(&ek_cert, &ek_pub, &store);
        assert!(matches!(r, Err(EkCertError::Expired)));
    }

    #[test]
    fn verify_ek_cert_rejects_not_yet_valid() {
        let ca = make_ca();
        let store = trust_store_from_pems(&[ca.cert.pem()], "vnotyet");
        // not_before well in the future.
        let (ek_cert, ek_pub) = make_ek_leaf(&ca, 3650, 7300);
        let r = verify_ek_cert_chain(&ek_cert, &ek_pub, &store);
        assert!(matches!(r, Err(EkCertError::NotYetValid)));
    }

    #[test]
    fn verify_ek_cert_rejects_untrusted_issuer() {
        let ca = make_ca();
        let other_ca = make_ca();
        // Trust store has only the *other* CA; leaf is signed by `ca`.
        let store = trust_store_from_pems(&[other_ca.cert.pem()], "vuntrusted");
        let (ek_cert, ek_pub) = make_ek_leaf(&ca, -365, 3650);
        let r = verify_ek_cert_chain(&ek_cert, &ek_pub, &store);
        assert!(
            matches!(
                r,
                Err(EkCertError::UntrustedIssuer | EkCertError::SignatureInvalid)
            ),
            "got {r:?}"
        );
    }

    #[test]
    fn verify_ek_cert_chain_succeeds() {
        let ca = make_ca();
        let store = trust_store_from_pems(&[ca.cert.pem()], "vok");
        let (ek_cert, ek_pub) = make_ek_leaf(&ca, -365, 3650);
        let r = verify_ek_cert_chain(&ek_cert, &ek_pub, &store);
        assert!(r.is_ok(), "expected Ok, got {r:?}");
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tpm_trust_test_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn trust_store_empty_dir_errors() {
        let d = tmpdir("empty");
        let r = TpmTrustStore::from_directory(&d);
        assert!(matches!(r, Err(TrustStoreError::NoCertsFound)));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn trust_store_missing_dir_is_io_error() {
        let r = TpmTrustStore::from_directory("/no/such/path/xyz");
        assert!(matches!(r, Err(TrustStoreError::Io(_))));
    }

    #[test]
    fn trust_store_ignores_unknown_extensions() {
        // A dir with only a .txt file → no candidate certs → NoCertsFound.
        let d = tmpdir("txtonly");
        fs::write(d.join("readme.txt"), b"hello").unwrap();
        let r = TpmTrustStore::from_directory(&d);
        assert!(matches!(r, Err(TrustStoreError::NoCertsFound)));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn trust_store_rejects_malformed_pem() {
        let d = tmpdir("badpem");
        fs::write(
            d.join("bad.pem"),
            b"-----BEGIN CERTIFICATE-----\nnotbase64\n-----END CERTIFICATE-----\n",
        )
        .unwrap();
        let r = TpmTrustStore::from_directory(&d);
        assert!(matches!(r, Err(TrustStoreError::InvalidCert { .. })));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn trust_store_rejects_malformed_der() {
        let d = tmpdir("badder");
        fs::write(d.join("bad.der"), b"\x00\x01\x02not a cert").unwrap();
        let r = TpmTrustStore::from_directory(&d);
        assert!(matches!(r, Err(TrustStoreError::InvalidCert { .. })));
        let _ = fs::remove_dir_all(&d);
    }
}
