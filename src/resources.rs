//! Resource control messages. Binary bytes are transferred by runtime adapters.

use serde::{Deserialize, Serialize};

use crate::format_calls::{ChunkRef, ResourceId, SessionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceStat {
    pub length: Option<u64>,
    pub mime_type: Option<String>,
    pub readable: bool,
    pub writable: bool,
    pub seekable: bool,
    pub revision: Option<String>,
    pub finished: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRead {
    pub chunk: ChunkRef,
    pub bytes: u64,
    pub eof: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    content = "input",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ResourceCall {
    Stat(ResourceRequest),
    Read(ReadRequest),
    ReadAt(ReadAtRequest),
    Seek(SeekRequest),
    Close(ResourceRequest),
    CreateOutput(CreateOutputRequest),
    Finish(ResourceRequest),
    ReleaseChunk(ChunkRequest),
    CreateSession(EmptyRequest),
    CheckSession(SessionRequest),
    CloseSession(SessionRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    pub resource: ResourceId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub resource: ResourceId,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadAtRequest {
    pub resource: ResourceId,
    pub offset: u64,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeekRequest {
    pub resource: ResourceId,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateOutputRequest {
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkRequest {
    pub chunk: ChunkRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionRequest {
    pub session_id: SessionId,
}
