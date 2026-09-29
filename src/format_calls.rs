//! Control messages for a declared `format_handler` operation.
//!
//! Resource and session identifiers are opaque Host-issued strings. Their
//! ownership (instance, generation, principal, scope and mode) is checked by
//! the Host on every operation; parsing a string never grants access. Media
//! bytes and large covers stay in Host resources, outside control JSON.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::format::{
    AssetRef, FormatHandlerCap, FormatMetadata, FormatOperation, InvalidFormatCapability,
};

/// Largest integer exactly representable by JavaScript Number.
pub const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
pub const MAX_METADATA_READ_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_MEDIA_CHUNK_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChunkRef(pub String);

/// The Host grants a readable resource and makes at most `prefix_bytes`
/// available to the plugin for this probe invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeRequest {
    pub input: ResourceId,
    pub extension_hint: Option<String>,
    pub mime_hint: Option<String>,
    pub prefix_bytes: u64,
}

impl ProbeRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_resource(&self.input)?;
        bounded(
            self.prefix_bytes,
            MAX_METADATA_READ_BYTES,
            "probe prefix_bytes",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProbeResult {
    Match,
    NoMatch,
    /// Total requested prefix size, not an additional byte count.
    NeedMore {
        total_bytes: u64,
    },
}

impl ProbeResult {
    pub fn validate(&self, available_prefix_bytes: u64) -> Result<(), InvalidFormatCapability> {
        if let Self::NeedMore { total_bytes } = self {
            if *total_bytes <= available_prefix_bytes {
                return Err(invalid("need_more must increase the cumulative prefix"));
            }
            bounded(*total_bytes, MAX_METADATA_READ_BYTES, "probe total_bytes")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataReadSizeRequest {
    pub input: ResourceId,
    pub available_prefix_bytes: u64,
}

impl MetadataReadSizeRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_resource(&self.input)?;
        bounded(
            self.available_prefix_bytes,
            MAX_METADATA_READ_BYTES,
            "available metadata prefix",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataReadSizeResult {
    /// Total head bytes required; Host may decline the request.
    pub prefix_bytes: u64,
}

impl MetadataReadSizeResult {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        bounded(
            self.prefix_bytes,
            MAX_METADATA_READ_BYTES,
            "metadata prefix_bytes",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractMetadataRequest {
    pub input: ResourceId,
    pub extract_cover: bool,
}

impl ExtractMetadataRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_resource(&self.input)
    }
}

/// Used only for *patches*. `Missing` omits the field, `Clear` emits null,
/// and `Set(value)` emits the value. Serde's default Option<Option<T>> would
/// collapse absent and null during deserialization.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PatchField<T> {
    #[default]
    Missing,
    Clear,
    Set(T),
}

impl<T> PatchField<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
}

impl<T: Serialize> Serialize for PatchField<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Missing => Err(serde::ser::Error::custom(
                "a missing patch field must be omitted",
            )),
            Self::Clear => serializer.serialize_none(),
            Self::Set(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for PatchField<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            None => Self::Clear,
            Some(value) => Self::Set(value),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataPatch {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub title: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub album: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub artist: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub album_artist: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub narrator: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub description: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub genre: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub track_number: PatchField<u32>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub tags: PatchField<Vec<String>>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    pub cover: PatchField<AssetRef>,
}

impl MetadataPatch {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        if [
            self.title.is_missing(),
            self.album.is_missing(),
            self.artist.is_missing(),
            self.album_artist.is_missing(),
            self.narrator.is_missing(),
            self.description.is_missing(),
            self.genre.is_missing(),
            self.track_number.is_missing(),
            self.tags.is_missing(),
            self.cover.is_missing(),
        ]
        .into_iter()
        .all(|missing| missing)
        {
            return Err(invalid("metadata patch must change at least one field"));
        }
        if matches!(self.track_number, PatchField::Set(0)) {
            return Err(invalid("track_number must be positive"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteMetadataRequest {
    pub input: ResourceId,
    /// Host-issued staging output; finish and authorized commit are separate.
    pub output: ResourceId,
    pub source_revision: String,
    pub patch: MetadataPatch,
}

impl WriteMetadataRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_resource(&self.input)?;
        validate_resource(&self.output)?;
        if self.input == self.output || self.source_revision.is_empty() {
            return Err(invalid(
                "metadata write needs separate staging output and source revision",
            ));
        }
        self.patch.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WriteMetadataResult {
    pub bytes_written: u64,
}

impl WriteMetadataResult {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        bounded(self.bytes_written, MAX_SAFE_INTEGER, "written bytes")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenDecryptRequest {
    pub input: ResourceId,
}

impl OpenDecryptRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_resource(&self.input)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamDescriptor {
    pub format: String,
    pub mime_type: String,
    /// Plaintext length, when known.
    pub length: Option<u64>,
    pub seekable: bool,
}

impl StreamDescriptor {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        if self.format.trim().is_empty() || self.mime_type.trim().is_empty() {
            return Err(invalid("stream format and MIME must be nonempty"));
        }
        if let Some(length) = self.length {
            bounded(length, MAX_SAFE_INTEGER, "stream length")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenDecryptResult {
    pub session_id: SessionId,
    pub output: StreamDescriptor,
}

impl OpenDecryptResult {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_session(&self.session_id)?;
        self.output.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadChunkRequest {
    pub session_id: SessionId,
    pub max_bytes: u64,
}

impl ReadChunkRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_session(&self.session_id)?;
        if self.max_bytes == 0 {
            return Err(invalid("read_chunk requires nonzero max_bytes"));
        }
        bounded(
            self.max_bytes,
            MAX_MEDIA_CHUNK_BYTES,
            "read_chunk max_bytes",
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadChunkResult {
    /// Host-managed leased chunk; release it before the producer reuses it.
    pub chunk: Option<ChunkRef>,
    pub length: u64,
    pub eof: bool,
}

impl ReadChunkResult {
    pub fn validate(&self, requested_bytes: u64) -> Result<(), InvalidFormatCapability> {
        if self.length > requested_bytes {
            return Err(invalid("read_chunk returned more than requested"));
        }
        bounded(self.length, MAX_MEDIA_CHUNK_BYTES, "read_chunk length")?;
        if (self.length == 0) != self.chunk.is_none() {
            return Err(invalid("chunk presence must match nonzero length"));
        }
        if self.chunk.as_ref().is_some_and(|chunk| chunk.0.is_empty()) {
            return Err(invalid("chunk reference must be nonempty"));
        }
        if self.length == 0 && !self.eof {
            return Err(invalid("empty non-EOF read would cause a busy loop"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeekRequest {
    pub session_id: SessionId,
    /// Plaintext byte position, never a ciphertext offset or time in seconds.
    pub offset: u64,
}

impl SeekRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_session(&self.session_id)?;
        bounded(self.offset, MAX_SAFE_INTEGER, "seek offset")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeekResult {
    pub position: u64,
}

impl SeekResult {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        bounded(self.position, MAX_SAFE_INTEGER, "seek position")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndSessionRequest {
    pub session_id: SessionId,
}

impl EndSessionRequest {
    pub fn validate(&self) -> Result<(), InvalidFormatCapability> {
        validate_session(&self.session_id)
    }
}

/// The fixed operation entry point used by every runtime adapter. Unknown
/// operations are rejected during deserialization; the declared capability
/// is checked before forwarding the call to a plugin instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "input", rename_all = "snake_case")]
pub enum FormatCall {
    Probe(ProbeRequest),
    GetMetadataReadSize(MetadataReadSizeRequest),
    ExtractMetadata(ExtractMetadataRequest),
    WriteMetadata(Box<WriteMetadataRequest>),
    OpenDecrypt(OpenDecryptRequest),
    ReadChunk(ReadChunkRequest),
    Seek(SeekRequest),
    Close(EndSessionRequest),
    Cancel(EndSessionRequest),
}

impl FormatCall {
    pub fn operation(&self) -> FormatOperation {
        match self {
            Self::Probe(_) => FormatOperation::Probe,
            Self::GetMetadataReadSize(_) => FormatOperation::GetMetadataReadSize,
            Self::ExtractMetadata(_) => FormatOperation::ExtractMetadata,
            Self::WriteMetadata(_) => FormatOperation::WriteMetadata,
            Self::OpenDecrypt(_) => FormatOperation::OpenDecrypt,
            Self::ReadChunk(_) => FormatOperation::ReadChunk,
            Self::Seek(_) => FormatOperation::Seek,
            Self::Close(_) => FormatOperation::Close,
            Self::Cancel(_) => FormatOperation::Cancel,
        }
    }

    pub fn validate(&self, declared: &FormatHandlerCap) -> Result<(), InvalidFormatCapability> {
        declared.validate()?;
        if !declared.supports(self.operation()) {
            return Err(invalid(format!(
                "format handler does not declare {:?}",
                self.operation()
            )));
        }
        match self {
            Self::Probe(request) => request.validate(),
            Self::GetMetadataReadSize(request) => request.validate(),
            Self::ExtractMetadata(request) => request.validate(),
            Self::WriteMetadata(request) => request.validate(),
            Self::OpenDecrypt(request) => request.validate(),
            Self::ReadChunk(request) => request.validate(),
            Self::Seek(request) => request.validate(),
            Self::Close(request) | Self::Cancel(request) => request.validate(),
        }
    }
}

/// Output for the selected operation. The Host must also validate the
/// concrete payload against its request, budget and current resource scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "data", rename_all = "snake_case")]
pub enum FormatOutput {
    Probe(ProbeResult),
    GetMetadataReadSize(MetadataReadSizeResult),
    ExtractMetadata(Box<FormatMetadata>),
    WriteMetadata(WriteMetadataResult),
    OpenDecrypt(OpenDecryptResult),
    ReadChunk(ReadChunkResult),
    Seek(SeekResult),
    Close,
    Cancel,
}

impl FormatOutput {
    pub fn operation(&self) -> FormatOperation {
        match self {
            Self::Probe(_) => FormatOperation::Probe,
            Self::GetMetadataReadSize(_) => FormatOperation::GetMetadataReadSize,
            Self::ExtractMetadata(_) => FormatOperation::ExtractMetadata,
            Self::WriteMetadata(_) => FormatOperation::WriteMetadata,
            Self::OpenDecrypt(_) => FormatOperation::OpenDecrypt,
            Self::ReadChunk(_) => FormatOperation::ReadChunk,
            Self::Seek(_) => FormatOperation::Seek,
            Self::Close => FormatOperation::Close,
            Self::Cancel => FormatOperation::Cancel,
        }
    }

    pub fn validate_for(&self, call: &FormatCall) -> Result<(), InvalidFormatCapability> {
        if self.operation() != call.operation() {
            return Err(invalid("format response operation differs from request"));
        }
        match (self, call) {
            (Self::Probe(result), FormatCall::Probe(request)) => {
                result.validate(request.prefix_bytes)
            }
            (Self::GetMetadataReadSize(result), _) => result.validate(),
            (Self::ExtractMetadata(result), _) => result.validate(),
            (Self::WriteMetadata(result), _) => result.validate(),
            (Self::OpenDecrypt(result), _) => result.validate(),
            (Self::ReadChunk(result), FormatCall::ReadChunk(request)) => {
                result.validate(request.max_bytes)
            }
            (Self::Seek(result), _) => result.validate(),
            (Self::Close | Self::Cancel, _) => Ok(()),
            _ => Err(invalid("format response payload differs from request")),
        }
    }
}

fn validate_resource(resource: &ResourceId) -> Result<(), InvalidFormatCapability> {
    if resource.0.is_empty() {
        return Err(invalid("resource ID must be nonempty"));
    }
    Ok(())
}

fn validate_session(session: &SessionId) -> Result<(), InvalidFormatCapability> {
    if session.0.is_empty() {
        return Err(invalid("session ID must be nonempty"));
    }
    Ok(())
}

fn bounded(value: u64, maximum: u64, field: &str) -> Result<(), InvalidFormatCapability> {
    if value > maximum {
        return Err(invalid(format!("{field} exceeds {maximum}")));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> InvalidFormatCapability {
    InvalidFormatCapability(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_value};

    #[test]
    fn patch_distinguishes_absent_clear_and_set_across_roundtrip() {
        let patch: MetadataPatch =
            serde_json::from_value(json!({"title": null, "tags": [], "track_number": 2})).unwrap();
        assert_eq!(patch.title, PatchField::Clear);
        assert_eq!(patch.album, PatchField::Missing);
        assert_eq!(patch.tags, PatchField::Set(vec![]));
        patch.validate().unwrap();
        let encoded = to_value(&patch).unwrap();
        assert_eq!(
            encoded,
            json!({"title": null, "tags": [], "track_number": 2})
        );
        assert!(MetadataPatch::default().validate().is_err());
        assert!(serde_json::from_value::<MetadataPatch>(json!({"duration": 2})).is_err());
        assert!(
            serde_json::from_value::<MetadataPatch>(json!({"track_number": 0}))
                .unwrap()
                .validate()
                .is_err()
        );
    }

    #[test]
    fn probe_and_metadata_reads_reject_over_budget_and_repeated_need_more() {
        let input = ProbeRequest {
            input: ResourceId("host:1".into()),
            extension_hint: Some("xm".into()),
            mime_hint: None,
            prefix_bytes: 4096,
        };
        input.validate().unwrap();
        let result = ProbeResult::NeedMore { total_bytes: 8192 };
        result.validate(input.prefix_bytes).unwrap();
        assert!(result.validate(8192).is_err());
        assert!(
            ProbeResult::NeedMore {
                total_bytes: MAX_METADATA_READ_BYTES + 1
            }
            .validate(4096)
            .is_err()
        );
        assert!(
            MetadataReadSizeResult {
                prefix_bytes: MAX_METADATA_READ_BYTES + 1
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn stream_chunks_and_offsets_are_bounded() {
        let request = ReadChunkRequest {
            session_id: SessionId("session:1".into()),
            max_bytes: MAX_MEDIA_CHUNK_BYTES,
        };
        request.validate().unwrap();
        let data = ReadChunkResult {
            chunk: Some(ChunkRef("lease:1".into())),
            length: 12,
            eof: false,
        };
        data.validate(request.max_bytes).unwrap();
        assert!(data.validate(10).is_err());
        assert!(
            ReadChunkResult {
                chunk: None,
                length: 0,
                eof: false
            }
            .validate(request.max_bytes)
            .is_err()
        );
        assert!(
            ReadChunkResult {
                chunk: None,
                length: 0,
                eof: true
            }
            .validate(request.max_bytes)
            .is_ok()
        );
        assert!(
            SeekRequest {
                session_id: request.session_id,
                offset: MAX_SAFE_INTEGER + 1
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn control_dto_rejects_unknown_fields_and_requires_stream_description() {
        assert!(
            serde_json::from_value::<OpenDecryptRequest>(json!({"input": "host:1", "path": "C:"}))
                .is_err()
        );
        assert!(
            StreamDescriptor {
                format: String::new(),
                mime_type: "audio/mpeg".into(),
                length: None,
                seekable: false
            }
            .validate()
            .is_err()
        );
        assert!(
            StreamDescriptor {
                format: "mp3".into(),
                mime_type: "audio/mpeg".into(),
                length: Some(MAX_SAFE_INTEGER + 1),
                seekable: true
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn extracted_metadata_is_an_independent_full_result() {
        assert!(serde_json::from_value::<FormatMetadata>(json!({"title": "partial"})).is_err());
    }

    #[test]
    fn metadata_write_only_targets_staging_and_requires_a_revision() {
        let patch = MetadataPatch {
            title: PatchField::Clear,
            ..Default::default()
        };
        let request = WriteMetadataRequest {
            input: ResourceId("source".into()),
            output: ResourceId("staging".into()),
            source_revision: "etag".into(),
            patch,
        };
        request.validate().unwrap();
        let mut invalid = request;
        invalid.output = invalid.input.clone();
        assert!(invalid.validate().is_err());
        invalid.output = ResourceId("staging".into());
        invalid.source_revision.clear();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn declared_operations_gate_calls_and_responses_on_all_runtimes() {
        let declared = FormatHandlerCap {
            id: "special.audio".into(),
            extensions: vec!["special".into()],
            operations: vec![FormatOperation::Probe, FormatOperation::ExtractMetadata],
        };
        let probe: FormatCall = serde_json::from_value(json!({
            "operation": "probe",
            "input": {"input": "host:1", "extension_hint": "special",
                      "mime_hint": null, "prefix_bytes": 4096}
        }))
        .unwrap();
        probe.validate(&declared).unwrap();
        FormatOutput::Probe(ProbeResult::NeedMore { total_bytes: 8192 })
            .validate_for(&probe)
            .unwrap();
        assert!(
            FormatOutput::Probe(ProbeResult::NeedMore { total_bytes: 4096 })
                .validate_for(&probe)
                .is_err()
        );
        assert!(FormatOutput::Close.validate_for(&probe).is_err());
        assert!(
            FormatCall::Close(EndSessionRequest {
                session_id: SessionId("session:1".into())
            })
            .validate(&declared)
            .is_err()
        );
        assert!(
            serde_json::from_value::<FormatCall>(json!({"operation":"get_stream_url","input":{}}))
                .is_err()
        );
    }
}
