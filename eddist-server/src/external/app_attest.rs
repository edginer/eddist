use std::io::Cursor;

use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use rustls_pki_types::{CertificateDer, TrustAnchor, UnixTime, pem::PemObject};
use serde::{Deserialize, de::DeserializeOwned};
use serde_bytes::ByteBuf;
use sha2::{Digest, Sha256};
use webpki::{EndEntityCert, ExtendedKeyUsageValidator, KeyPurposeIdIter};
use x509_parser::{oid_registry::Oid, parse_x509_certificate};

const APPLE_ROOT: &[u8] = include_bytes!("../../resources/apple-app-attestation-root.pem");
const AUTH_DATA_HEADER_LEN: usize = 37;
const ATTESTED_CREDENTIAL_FLAG: u8 = 0x40;

#[derive(Debug, thiserror::Error)]
#[error("invalid App Attest proof")]
pub(crate) struct InvalidProof;

#[derive(Clone)]
pub(crate) struct AppAttestVerifier {
    root: TrustAnchor<'static>,
    rp_id: [u8; 32],
    development: bool,
}

#[derive(Deserialize)]
struct Attestation {
    fmt: String,
    #[serde(rename = "attStmt")]
    statement: AttestationStatement,
    #[serde(rename = "authData")]
    auth_data: ByteBuf,
}

#[derive(Deserialize)]
struct AttestationStatement {
    x5c: Vec<ByteBuf>,
    receipt: ByteBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Assertion {
    authenticator_data: ByteBuf,
    signature: ByteBuf,
}

#[derive(Deserialize)]
struct Extensions {
    apple_validation_category_01: ByteBuf,
    apple_bundle_version_01: String,
}

// App Attest uses a dedicated trust root and a certificate nonce extension.
// Its certificates do not use a TLS extended key usage.
struct AppAttestKeyUsage;

impl ExtendedKeyUsageValidator for AppAttestKeyUsage {
    fn validate(&self, usages: KeyPurposeIdIter<'_, '_>) -> Result<(), webpki::Error> {
        for usage in usages {
            usage?;
        }
        Ok(())
    }
}

impl AppAttestVerifier {
    pub(crate) fn new(app_id: &str, development: bool) -> anyhow::Result<Self> {
        let root = CertificateDer::from_pem_slice(APPLE_ROOT)?;
        let root = webpki::anchor_from_trusted_cert(&root)?.to_owned();
        Ok(Self {
            root,
            rp_id: Sha256::digest(app_id).into(),
            development,
        })
    }

    pub(crate) fn verify_attestation(
        &self,
        proof: &[u8],
        key_id: &[u8; 32],
        client_data_hash: &[u8; 32],
        now: UnixTime,
    ) -> Result<[u8; 65], InvalidProof> {
        let attestation: Attestation = decode_cbor(proof)?;
        let certificates = &attestation.statement.x5c;
        if attestation.fmt != "apple-appattest"
            || !(2..=3).contains(&certificates.len())
            || attestation.statement.receipt.is_empty()
        {
            return Err(InvalidProof);
        }
        let auth_data = &attestation.auth_data;
        if self.read_counter(auth_data)? != 0 || auth_data[32] & ATTESTED_CREDENTIAL_FLAG == 0 {
            return Err(InvalidProof);
        }
        let extensions =
            self.after_attested_credential(&auth_data[AUTH_DATA_HEADER_LEN..], key_id)?;
        self.verify_extensions(extensions)?;

        let leaf_der = CertificateDer::from(certificates[0].as_ref());
        let leaf = EndEntityCert::try_from(&leaf_der).map_err(|_| InvalidProof)?;
        let intermediates = certificates[1..]
            .iter()
            .map(|cert| CertificateDer::from(cert.as_ref()))
            .collect::<Vec<_>>();
        leaf.verify_for_usage(
            webpki::ALL_VERIFICATION_ALGS,
            std::slice::from_ref(&self.root),
            &intermediates,
            now,
            AppAttestKeyUsage,
            None,
            None,
        )
        .map_err(|_| InvalidProof)?;

        let (remaining, certificate) =
            parse_x509_certificate(leaf_der.as_ref()).map_err(|_| InvalidProof)?;
        if !remaining.is_empty() {
            return Err(InvalidProof);
        }
        let oid = Oid::from(&[1, 2, 840, 113635, 100, 8, 2]).map_err(|_| InvalidProof)?;
        let extension = certificate
            .get_extension_unique(&oid)
            .map_err(|_| InvalidProof)?
            .ok_or(InvalidProof)?;
        // DER SEQUENCE containing a context-specific [1] holding the 32-byte OCTET STRING.
        let der_prefix = [0x30, 0x24, 0xa1, 0x22, 0x04, 0x20];
        if extension.value.strip_prefix(&der_prefix) != Some(&nonce(auth_data, client_data_hash)) {
            return Err(InvalidProof);
        }

        let raw_key = &certificate.public_key().subject_public_key;
        let public_key: [u8; 65] = raw_key.data.as_ref().try_into().map_err(|_| InvalidProof)?;
        if raw_key.unused_bits != 0 || <[u8; 32]>::from(Sha256::digest(public_key)) != *key_id {
            return Err(InvalidProof);
        }
        VerifyingKey::from_sec1_bytes(&public_key).map_err(|_| InvalidProof)?;
        Ok(public_key)
    }

    pub(crate) fn verify_assertion(
        &self,
        proof: &[u8],
        public_key: &[u8],
        client_data_hash: &[u8; 32],
        previous_counter: u32,
    ) -> Result<u32, InvalidProof> {
        let assertion: Assertion = decode_cbor(proof)?;
        let auth_data = &assertion.authenticator_data;
        let counter = self.read_counter(auth_data)?;
        if counter <= previous_counter {
            return Err(InvalidProof);
        }
        // Older iOS releases can set AT while omitting the credential data.
        let trailing = &auth_data[AUTH_DATA_HEADER_LEN..];
        let extensions = if auth_data[32] & ATTESTED_CREDENTIAL_FLAG != 0
            && trailing.starts_with(b"appattest")
        {
            self.after_attested_credential(trailing, &Sha256::digest(public_key).into())?
        } else {
            trailing
        };
        self.verify_extensions(extensions)?;
        let key = VerifyingKey::from_sec1_bytes(public_key).map_err(|_| InvalidProof)?;
        let signature = Signature::from_der(&assertion.signature).map_err(|_| InvalidProof)?;
        key.verify(&nonce(auth_data, client_data_hash), &signature)
            .map_err(|_| InvalidProof)?;
        Ok(counter)
    }

    fn after_attested_credential<'a>(
        &self,
        bytes: &'a [u8],
        key_id: &[u8; 32],
    ) -> Result<&'a [u8], InvalidProof> {
        let aaguid = bytes.get(..16).ok_or(InvalidProof)?;
        let valid_environment = if self.development {
            matches!(aaguid, b"appattestdevelop" | b"appattestsandbox")
        } else {
            aaguid == b"appattest\0\0\0\0\0\0\0"
        };
        if !valid_environment
            || bytes.get(16..18) != Some(&[0, 32])
            || bytes.get(18..50) != Some(key_id)
        {
            return Err(InvalidProof);
        }
        let cose_and_extensions = bytes.get(50..).ok_or(InvalidProof)?;
        let mut cursor = Cursor::new(cose_and_extensions);
        if !matches!(
            decode_value::<ciborium::Value>(&mut cursor)?,
            ciborium::Value::Map(_)
        ) {
            return Err(InvalidProof);
        }
        Ok(&cose_and_extensions[cursor.position() as usize..])
    }

    fn read_counter(&self, auth_data: &[u8]) -> Result<u32, InvalidProof> {
        if auth_data.get(..32) != Some(&self.rp_id) {
            return Err(InvalidProof);
        }
        let counter = auth_data
            .get(33..AUTH_DATA_HEADER_LEN)
            .ok_or(InvalidProof)?;
        Ok(u32::from_be_bytes(
            counter.try_into().map_err(|_| InvalidProof)?,
        ))
    }

    fn verify_extensions(&self, bytes: &[u8]) -> Result<(), InvalidProof> {
        // Earlier versions of iOS issue App Attest proofs without these extensions.
        if bytes.is_empty() {
            return Ok(());
        }
        let extensions: Extensions = decode_cbor(bytes)?;
        let category = u32::from_le_bytes(
            extensions
                .apple_validation_category_01
                .as_ref()
                .try_into()
                .map_err(|_| InvalidProof)?,
        );
        let allowed_category = if self.development {
            category == 3
        } else {
            matches!(category, 2 | 4)
        };
        if !allowed_category || extensions.apple_bundle_version_01.is_empty() {
            return Err(InvalidProof);
        }
        Ok(())
    }
}

fn decode_value<T: DeserializeOwned>(reader: &mut Cursor<&[u8]>) -> Result<T, InvalidProof> {
    ciborium::de::from_reader_with_recursion_limit(reader, 12).map_err(|_| InvalidProof)
}

fn decode_cbor<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, InvalidProof> {
    let mut reader = Cursor::new(bytes);
    let value = decode_value(&mut reader)?;
    if reader.position() as usize != bytes.len() {
        return Err(InvalidProof);
    }
    Ok(value)
}

fn nonce(auth_data: &[u8], client_data_hash: &[u8; 32]) -> [u8; 32] {
    Sha256::new()
        .chain_update(auth_data)
        .chain_update(client_data_hash)
        .finalize()
        .into()
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use ciborium::Value;
    use p256::ecdsa::{SigningKey, signature::Signer};

    use super::*;

    #[test]
    fn verifies_assertion_with_attested_credential_data_before_extensions() {
        let verifier = AppAttestVerifier::new("ABC1234567.com.example.app", true).unwrap();
        let signing_key = SigningKey::from_slice(&[1; 32]).unwrap();
        let public_key = signing_key.verifying_key().to_sec1_point(false);
        let client_data_hash = [2; 32];
        let mut auth_data = verifier.rp_id.to_vec();
        auth_data.push(ATTESTED_CREDENTIAL_FLAG | 0x80);
        auth_data.extend(2u32.to_be_bytes());
        auth_data.extend(b"appattestdevelop");
        auth_data.extend(32u16.to_be_bytes());
        auth_data.extend(Sha256::digest(public_key.as_bytes()));
        ciborium::into_writer(&Value::Map(Vec::new()), &mut auth_data).unwrap();

        let make_assertion = |category: u32| {
            let mut auth_data = auth_data.clone();
            let extensions = Value::Map(vec![
                (
                    Value::Text("apple_validation_category_01".into()),
                    Value::Bytes(category.to_le_bytes().to_vec()),
                ),
                (
                    Value::Text("apple_bundle_version_01".into()),
                    Value::Text("1".into()),
                ),
            ]);
            ciborium::into_writer(&extensions, &mut auth_data).unwrap();
            let signature: Signature = signing_key.sign(&nonce(&auth_data, &client_data_hash));
            let assertion = Value::Map(vec![
                (
                    Value::Text("authenticatorData".into()),
                    Value::Bytes(auth_data),
                ),
                (
                    Value::Text("signature".into()),
                    Value::Bytes(signature.to_der().as_bytes().to_vec()),
                ),
            ]);
            let mut proof = Vec::new();
            ciborium::into_writer(&assertion, &mut proof).unwrap();
            proof
        };
        assert_eq!(
            verifier
                .verify_assertion(
                    &make_assertion(3),
                    public_key.as_bytes(),
                    &client_data_hash,
                    1
                )
                .unwrap(),
            2
        );
        assert!(
            verifier
                .verify_assertion(
                    &make_assertion(4),
                    public_key.as_bytes(),
                    &client_data_hash,
                    1
                )
                .is_err()
        );
    }

    #[test]
    fn verifies_real_apple_attestation_and_assertion() {
        // Converted from devicecheck-appattest's iOS 14.4 fixture (Apache-2.0):
        // attestation and assertion extracted as base64 lines.
        // https://github.com/veehaitch/devicecheck-appattest/blob/main/src/test/resources/ios-14.4.yaml
        let mut fixtures = include_str!("../../tests/fixtures/ios-14.4-app-attest.base64").lines();
        let attestation = STANDARD.decode(fixtures.next().unwrap()).unwrap();
        let assertion = STANDARD.decode(fixtures.next().unwrap()).unwrap();
        let app_id = "6MURL8TA57.de.vincent-haupert.apple-appattest-poc";
        let key_id = STANDARD
            .decode("YmbJO4x5nEHUvncp9zdWuVZjNBEMgJn3cdSToAXQe3M=")
            .unwrap()
            .try_into()
            .unwrap();
        let hash = Sha256::digest(b"wurzelpfropf").into();
        let issued_at = chrono::DateTime::parse_from_rfc3339("2021-01-23T12:13:33.335Z").unwrap();
        let date = UnixTime::since_unix_epoch(std::time::Duration::from_secs(
            issued_at.timestamp() as u64,
        ));
        let verifier = AppAttestVerifier::new(app_id, true).unwrap();
        let public_key = verifier
            .verify_attestation(&attestation, &key_id, &hash, date)
            .unwrap();
        assert_eq!(
            verifier
                .verify_assertion(&assertion, &public_key, &hash, 0)
                .unwrap(),
            1
        );
        assert!(
            verifier
                .verify_assertion(&assertion, &public_key, &hash, 1)
                .is_err()
        );
        assert!(
            verifier
                .verify_assertion(&assertion, &public_key, &[0; 32], 0)
                .is_err()
        );
        assert!(
            verifier
                .verify_attestation(&attestation, &key_id, &[0; 32], date)
                .is_err()
        );
        assert!(
            verifier
                .verify_attestation(&attestation, &key_id, &hash, UnixTime::now())
                .is_err()
        );
        let production = AppAttestVerifier::new(app_id, false).unwrap();
        assert!(
            production
                .verify_attestation(&attestation, &key_id, &hash, date)
                .is_err()
        );
    }
}
