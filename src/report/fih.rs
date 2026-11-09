//! Fact primitives — ev's output envelope for neXus consumption.
//!
//! Every output ev produces (verification results, synthesis reports) is
//! wrapped in a Fact. This is the only interface between ev and any
//! downstream colony.
//!
//! # Design
//!
//! The Fact struct uses `Vec<u8>` (blob) for its payload. No schema is
//! embedded at this layer — consumers interpret the blob as they see fit
//! (JSON, CBOR, protobuf, or raw binary). The `fact_type` field serves
//! as a discriminator for consumer-side deserialization.
//!
//! Extension layers (e.g., Nexus ingestion) can attach schema hints via
//! the `extra` field or through side-channel negotiation (content-type
//! header, file extension, etc.).

use serde::{Deserialize, Serialize};

/// The payload travels as base64 rather than as a JSON array of byte values.
///
/// A byte costs one and a third characters instead of six, which is the
/// difference between a 2 GB record and a 330 MB one at a million points.
/// The blob stays opaque: the encoding is the envelope's business, and a
/// consumer decodes it before interpreting `fact_type`.
pub mod base64 {
    use serde::{Deserialize, Deserializer, Serializer};

    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }

    pub fn decode(text: &str) -> Option<Vec<u8>> {
        let bytes = text.as_bytes();
        if !bytes.len().is_multiple_of(4) {
            return None;
        }
        let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
        for chunk in bytes.chunks(4) {
            let mut n: u32 = 0;
            let mut padding = 0;
            for &c in chunk {
                let v = match c {
                    b'A'..=b'Z' => c - b'A',
                    b'a'..=b'z' => c - b'a' + 26,
                    b'0'..=b'9' => c - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    b'=' => {
                        padding += 1;
                        0
                    }
                    _ => return None,
                };
                if padding > 0 && c != b'=' {
                    return None;
                }
                n = (n << 6) | v as u32;
            }
            out.push((n >> 16) as u8);
            if chunk[2] != b'=' {
                out.push((n >> 8) as u8);
            }
            if chunk[3] != b'=' {
                out.push(n as u8);
            }
        }
        Some(out)
    }

    #[allow(clippy::ptr_arg)] // serde's `with` hands the field's own type.
    pub fn serialize<S: Serializer>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(deserializer)?;
        decode(&text).ok_or_else(|| serde::de::Error::custom("payload is not base64"))
    }
}

/// A validated observation — immutable once committed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    /// Stable type tag: "verification_result", "synthesis_result", etc.
    pub fact_type: String,
    /// Origin identifier: "ev/0.1.0", "ev/synthesis/yosys".
    pub origin: String,
    /// Target module or instruction identifier.
    pub target: String,
    /// Opaque payload blob. Consumers interpret based on fact_type.
    /// Common encodings: JSON, CBOR, msgpack, raw binary. Serialized as
    /// base64 so the envelope does not spend six characters on a byte.
    #[serde(with = "base64")]
    pub payload: Vec<u8>,
    /// ISO 8601 timestamp of observation.
    pub timestamp: String,
    /// Optional hash of the parent Fact that triggered this observation.
    pub parent_fact_id: Option<String>,
}

impl Fact {
    /// Create a Fact with the current UTC timestamp.
    ///
    /// `payload` is an opaque byte vector — no schema assumed.
    pub fn new(
        fact_type: impl Into<String>,
        origin: impl Into<String>,
        target: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Self {
            fact_type: fact_type.into(),
            origin: origin.into(),
            target: target.into(),
            payload,
            timestamp,
            parent_fact_id: None,
        }
    }
}
