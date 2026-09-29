//! Contract for special format extensions and bounded metadata extraction.
//!
//! A plugin advertises extensions and operations. The host must select a
//! candidate by declaration and probe result, and invoke only declared
//! operations. There are no format-specific branches in this crate.

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

/// The tagged declaration as it appears inside `capabilities` in plugin.yml.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormatCapabilityDeclaration {
    FormatHandler(FormatHandlerCap),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatHandlerCap {
    pub id: String,
    pub extensions: Vec<String>,
    pub operations: Vec<FormatOperation>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatOperation {
    Probe,
    GetMetadataReadSize,
    ExtractMetadata,
    WriteMetadata,
    OpenDecrypt,
    ReadChunk,
    Seek,
    Close,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidFormatCapability(pub String);

impl std::fmt::Display for InvalidFormatCapability {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.0)
    }
}

impl std::error::Error for InvalidFormatCapability {}

impl FormatHandlerCap {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(InvalidFormatCapability(
                "format_handler.id must be a nonempty safe identifier (<= 64 bytes)".into(),
            ));
        }
        if self.extensions.is_empty() {
            return Err(InvalidFormatCapability(
                "format_handler.extensions must not be empty".into(),
            ));
        }
        let mut extensions = HashSet::new();
        for extension in &self.extensions {
            if extension.is_empty()
                || extension.len() > 16
                || !extension
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                || !extensions.insert(extension.as_str())
            {
                return Err(InvalidFormatCapability(format!(
                    "invalid or duplicate format_handler extension: {extension}"
                )));
            }
        }
        let mut operations = HashSet::new();
        for operation in &self.operations {
            if !operations.insert(*operation) {
                return Err(InvalidFormatCapability(format!(
                    "duplicate format_handler operation: {operation:?}"
                )));
            }
        }
        if !operations.contains(&FormatOperation::Probe)
            || !(operations.contains(&FormatOperation::ExtractMetadata)
                || operations.contains(&FormatOperation::OpenDecrypt))
        {
            return Err(InvalidFormatCapability(
                "format_handler requires probe and either extract_metadata or open_decrypt".into(),
            ));
        }
        if operations.contains(&FormatOperation::GetMetadataReadSize)
            && !operations.contains(&FormatOperation::ExtractMetadata)
        {
            return Err(InvalidFormatCapability(
                "get_metadata_read_size requires extract_metadata".into(),
            ));
        }
        if operations.contains(&FormatOperation::WriteMetadata)
            && !operations.contains(&FormatOperation::ExtractMetadata)
        {
            return Err(InvalidFormatCapability(
                "write_metadata requires extract_metadata".into(),
            ));
        }
        let stream = [
            FormatOperation::OpenDecrypt,
            FormatOperation::ReadChunk,
            FormatOperation::Close,
            FormatOperation::Cancel,
        ];
        let declared_stream_ops = stream.iter().filter(|op| operations.contains(op)).count();
        if declared_stream_ops != 0 && declared_stream_ops != stream.len() {
            return Err(InvalidFormatCapability(
                "decryption requires open_decrypt, read_chunk, close and cancel together".into(),
            ));
        }
        if operations.contains(&FormatOperation::Seek)
            && !operations.contains(&FormatOperation::OpenDecrypt)
        {
            return Err(InvalidFormatCapability(
                "seek requires a decryption stream".into(),
            ));
        }
        Ok(())
    }

    pub fn supports(&self, operation: FormatOperation) -> bool {
        self.operations.contains(&operation)
    }
}

impl FormatOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Probe => "probe",
            Self::GetMetadataReadSize => "get_metadata_read_size",
            Self::ExtractMetadata => "extract_metadata",
            Self::WriteMetadata => "write_metadata",
            Self::OpenDecrypt => "open_decrypt",
            Self::ReadChunk => "read_chunk",
            Self::Seek => "seek",
            Self::Close => "close",
            Self::Cancel => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormatMetadata {
    #[serde(deserialize_with = "required_nullable")]
    pub title: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub album: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub artist: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub album_artist: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub narrator: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub description: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub genre: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub track_number: Option<u32>,
    pub tags: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub duration: Option<f64>,
    #[serde(deserialize_with = "required_nullable")]
    pub bitrate: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub sample_rate: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub channels: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub format: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub mime_type: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub cover: Option<AssetRef>,
}

// Option<T> alone treats a missing field like explicit null. Complete metadata
// results require all nullable fields to be present; patches use PatchField.
fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssetRef {
    HostAsset { id: String },
    RemoteUrl { url: String },
}

impl FormatMetadata {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        if self
            .duration
            .is_some_and(|duration| !duration.is_finite() || duration < 0.0)
        {
            return Err(InvalidFormatCapability(
                "format metadata duration must be finite and nonnegative".into(),
            ));
        }
        if self.track_number == Some(0)
            || self.bitrate == Some(0)
            || self.sample_rate == Some(0)
            || self.channels == Some(0)
        {
            return Err(InvalidFormatCapability(
                "format metadata numeric fields must be positive when present".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_capability(json: &str) -> Result<FormatHandlerCap, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn metadata_only_extension_needs_no_decrypt_operation() {
        let declared: FormatCapabilityDeclaration = serde_json::from_str(
            r#"{"kind":"format_handler","id":"format.handler","extensions":["xm"],"operations":["probe","get_metadata_read_size","extract_metadata"]}"#,
        )
        .unwrap();
        let FormatCapabilityDeclaration::FormatHandler(inner) = declared;
        inner.validate().unwrap();
        let capability = parse_capability(
            r#"{"id":"format.handler","extensions":["xm"],"operations":["probe","get_metadata_read_size","extract_metadata"]}"#,
        )
        .unwrap();
        capability.validate().unwrap();
        assert!(capability.supports(FormatOperation::ExtractMetadata));
        assert!(!capability.supports(FormatOperation::OpenDecrypt));
        assert!(serde_json::to_value(&capability).unwrap()["extensions"].is_array());
    }

    #[test]
    fn stream_extension_declares_complete_lifecycle() {
        let cap = parse_capability(
            r#"{"id":"encrypted.audio","extensions":["sample"],"operations":["probe","open_decrypt","read_chunk","close","cancel","seek"]}"#,
        )
        .unwrap();
        cap.validate().unwrap();
    }

    #[test]
    fn rejects_unknown_fields_operations_and_incomplete_decrypt() {
        assert!(parse_capability(
            r#"{"id":"a","extensions":["xm"],"operations":["probe","extract_metadata"],"extra":"ignored"}"#,
        )
        .is_err());
        assert!(
            parse_capability(
                r#"{"id":"a","extensions":["xm"],"operations":["probe","magic_decode"]}"#,
            )
            .is_err()
        );
        for operations in [
            vec![FormatOperation::Probe, FormatOperation::OpenDecrypt],
            vec![FormatOperation::Probe, FormatOperation::GetMetadataReadSize],
            vec![
                FormatOperation::Probe,
                FormatOperation::ExtractMetadata,
                FormatOperation::ExtractMetadata,
            ],
            vec![
                FormatOperation::Probe,
                FormatOperation::ExtractMetadata,
                FormatOperation::Seek,
            ],
        ] {
            assert!(
                FormatHandlerCap {
                    id: "example".into(),
                    extensions: vec!["xm".into()],
                    operations,
                }
                .validate()
                .is_err()
            );
        }
    }

    #[test]
    fn requires_unique_normalized_extension_names() {
        for extensions in [
            vec!["xm".to_string(), "xm".to_string()],
            vec![".xm".to_string()],
            vec!["XM".to_string()],
            vec!["../xm".to_string()],
        ] {
            assert!(
                FormatHandlerCap {
                    id: "example".into(),
                    extensions,
                    operations: vec![FormatOperation::Probe, FormatOperation::ExtractMetadata,],
                }
                .validate()
                .is_err()
            );
        }
    }

    #[test]
    fn metadata_schema_distinguishes_null_and_empty_tags() {
        let metadata = serde_json::from_value::<FormatMetadata>(serde_json::json!({
            "title": null, "album": null, "artist": "example", "album_artist": null,
            "narrator": null, "description": null, "genre": null,
            "track_number": null, "tags": [], "duration": 0.25,
            "bitrate": null, "sample_rate": null, "channels": null,
            "format": null, "mime_type": null, "cover": null
        }))
        .unwrap();
        metadata.validate().unwrap();
        assert_eq!(metadata.tags.len(), 0);
        let mut invalid = metadata;
        invalid.duration = Some(f64::NAN);
        assert!(invalid.validate().is_err());
        invalid.duration = Some(1.0);
        invalid.track_number = Some(0);
        assert!(invalid.validate().is_err());
    }
}
