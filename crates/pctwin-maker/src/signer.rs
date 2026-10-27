//! Reading who signed a Windows file from its signature (a PKCS #7 SignedData), in pure Rust.
//!
//! The `authenticode` crate checks the signature's shape and gives the digest the maker signed.
//! Its own certificate list panics on a signature without certificates, so the certificates are
//! read here from the same bytes. Nothing here decides trust: that is Windows' own check.

use authenticode::AuthenticodeSignature;
use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier};
use const_oid::ObjectIdentifier;
use der::asn1::{Ia5StringRef, PrintableStringRef, TeletexStringRef, Utf8StringRef};
use der::{Decode, Encode, SliceReader, Tag, Tagged};
use sha2::{Digest, Sha256 as Sha256Hasher};
use x509_cert::Certificate;
use x509_cert::attr::AttributeTypeAndValue;

use crate::facts::{Sha256, Subject, WindowsSigner};

/// The largest signature PCTwin reads. Real ones are a few tens of kilobytes.
pub const MAX_SIGNATURE: usize = 4 * 1024 * 1024;

/// A digest algorithm a Windows signature may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestAlg {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

/// A hash in progress, for whichever algorithm the signature names.
pub enum Hasher {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha384(sha2::Sha384),
    Sha512(sha2::Sha512),
}

impl Hasher {
    pub fn new(alg: DigestAlg) -> Self {
        match alg {
            DigestAlg::Sha1 => Hasher::Sha1(sha1::Sha1::new()),
            DigestAlg::Sha256 => Hasher::Sha256(sha2::Sha256::new()),
            DigestAlg::Sha384 => Hasher::Sha384(sha2::Sha384::new()),
            DigestAlg::Sha512 => Hasher::Sha512(sha2::Sha512::new()),
        }
    }

    pub fn update(&mut self, b: &[u8]) {
        match self {
            Hasher::Sha1(h) => h.update(b),
            Hasher::Sha256(h) => h.update(b),
            Hasher::Sha384(h) => h.update(b),
            Hasher::Sha512(h) => h.update(b),
        }
    }

    pub fn finish(self) -> Vec<u8> {
        match self {
            Hasher::Sha1(h) => h.finalize().to_vec(),
            Hasher::Sha256(h) => h.finalize().to_vec(),
            Hasher::Sha384(h) => h.finalize().to_vec(),
            Hasher::Sha512(h) => h.finalize().to_vec(),
        }
    }
}

/// A signature read: who signed, the digest they signed and its algorithm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub signer: Result<WindowsSigner, String>,
    pub alg: DigestAlg,
    pub signed_digest: Vec<u8>,
}

const OID_SHA1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.14.3.2.26");
const OID_SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
const OID_SHA384: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.2");
const OID_SHA512: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.3");

const OID_CN: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");
const OID_SERIAL: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.5");
const OID_C: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.6");
const OID_L: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.7");
const OID_ST: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.8");
const OID_O: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.10");
const OID_JUR_ST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.4.1.311.60.2.1.2");
const OID_JUR_C: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.4.1.311.60.2.1.3");

/// Reads a signature's bytes. Err when it is not a well-formed Authenticode signature.
pub fn read_signature(bytes: &[u8]) -> Result<Signed, String> {
    if bytes.len() > MAX_SIGNATURE {
        return Err("the signature is too large to be real".into());
    }
    let sig = AuthenticodeSignature::from_bytes(bytes)
        .map_err(|e| format!("the signature cannot be read ({e})"))?;
    let oid = sig.digest_algorithm().oid;
    let alg = match oid {
        OID_SHA1 => DigestAlg::Sha1,
        OID_SHA256 => DigestAlg::Sha256,
        OID_SHA384 => DigestAlg::Sha384,
        OID_SHA512 => DigestAlg::Sha512,
        _ => return Err(format!("the signature uses an unknown digest ({oid})")),
    };
    let expected_len = match alg {
        DigestAlg::Sha1 => 20,
        DigestAlg::Sha256 => 32,
        DigestAlg::Sha384 => 48,
        DigestAlg::Sha512 => 64,
    };
    if sig.digest().len() != expected_len {
        return Err("the signed digest has the wrong length".into());
    }
    Ok(Signed {
        signer: signer_of(bytes),
        alg,
        signed_digest: sig.digest().to_vec(),
    })
}

/// Finds the signing certificate named by the one signer, and reads its subject.
fn signer_of(bytes: &[u8]) -> Result<WindowsSigner, String> {
    let mut reader = SliceReader::new(bytes).map_err(|e| e.to_string())?;
    let info = ContentInfo::decode(&mut reader).map_err(|e| e.to_string())?;
    let signed: SignedData = info.content.decode_as().map_err(|e| e.to_string())?;
    let signer = signed
        .signer_infos
        .0
        .as_slice()
        .first()
        .ok_or("the signature names no signer")?;
    let SignerIdentifier::IssuerAndSerialNumber(id) = &signer.sid else {
        return Err("the signer is named by key only, which PCTwin does not read".into());
    };
    let certs = signed
        .certificates
        .as_ref()
        .ok_or("the signature carries no certificates")?;
    let mut found = certs.0.iter().filter_map(|c| match c {
        CertificateChoices::Certificate(c)
            if c.tbs_certificate.issuer == id.issuer
                && c.tbs_certificate.serial_number == id.serial_number =>
        {
            Some(c)
        }
        _ => None,
    });
    let cert = found
        .next()
        .ok_or("the signing certificate is missing from the signature")?;
    if found.next().is_some() {
        return Err("the signature carries two certificates for its signer".into());
    }
    signer_from_certificate(cert)
}

/// The maker named by one certificate.
pub fn signer_from_certificate(cert: &Certificate) -> Result<WindowsSigner, String> {
    let der = cert.to_der().map_err(|e| e.to_string())?;
    let mut subject = Subject::default();
    for rdn in cert.tbs_certificate.subject.0.iter() {
        for atv in rdn.0.iter() {
            let slot = match atv.oid {
                OID_CN => &mut subject.common_name,
                OID_O => &mut subject.organization,
                OID_L => &mut subject.locality,
                OID_ST => &mut subject.state,
                OID_C => &mut subject.country,
                OID_SERIAL => &mut subject.registration_number,
                OID_JUR_C => &mut subject.jurisdiction_country,
                OID_JUR_ST => &mut subject.jurisdiction_state,
                _ => continue,
            };
            let text = text_of(atv)?;
            match slot {
                Some(had) if *had != text => {
                    return Err("the certificate names its maker twice, differently".into());
                }
                _ => *slot = Some(text),
            }
        }
    }
    Ok(WindowsSigner {
        subject,
        certificate: Sha256(Sha256Hasher::digest(&der).into()),
        issuer: cert.tbs_certificate.issuer.to_string(),
    })
}

/// Reads a certificate's DER bytes (from Windows' own check) into the maker it names.
pub fn signer_from_der(der: &[u8]) -> Result<WindowsSigner, String> {
    let cert = Certificate::from_der(der).map_err(|e| e.to_string())?;
    signer_from_certificate(&cert)
}

fn text_of(atv: &AttributeTypeAndValue) -> Result<String, String> {
    let v = &atv.value;
    let s = match v.tag() {
        Tag::Utf8String => v
            .decode_as::<Utf8StringRef<'_>>()
            .map(|s| s.as_str().to_owned()),
        Tag::PrintableString => v
            .decode_as::<PrintableStringRef<'_>>()
            .map(|s| s.as_str().to_owned()),
        Tag::Ia5String => v
            .decode_as::<Ia5StringRef<'_>>()
            .map(|s| s.as_str().to_owned()),
        Tag::TeletexString => v
            .decode_as::<TeletexStringRef<'_>>()
            .map(|s| s.as_str().to_owned()),
        Tag::BmpString => v.decode_as::<der::asn1::BmpString>().map(|s| s.to_string()),
        other => {
            return Err(format!(
                "a name in the certificate has an unknown form ({other})"
            ));
        }
    };
    s.map_err(|e| format!("a name in the certificate cannot be read ({e})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_limit_is_exact() {
        let at_limit = read_signature(&vec![0u8; MAX_SIGNATURE]).unwrap_err();
        assert!(at_limit.contains("cannot be read"), "{at_limit}");
        let over = read_signature(&vec![0u8; MAX_SIGNATURE + 1]).unwrap_err();
        assert!(over.contains("too large"), "{over}");
    }
}
