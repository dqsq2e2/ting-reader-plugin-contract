//! Closed capability declarations shared by loaders, SDKs and packagers.
//!
//! Dynamic tool/task/event payloads have explicit, bounded JSON Schemas.
//! Static manifest objects never accept extension fields or entry-point aliases.

use crate::format::FormatHandlerCap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub type LocalizedText = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Capability {
    MetadataProvider(MetadataProviderCap),
    FormatHandler(FormatHandlerCap),
    ToolProvider(ToolProviderCap),
    HttpRoute(HttpRouteCap),
    UiExtension(UiExtensionCap),
    PluginStore(PluginStoreCap),
    ContentProcessor(ContentProcessorCap),
    TaskHandler(TaskHandlerCap),
    EventHandler(EventHandlerCap),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidCapability(pub String);

impl std::fmt::Display for InvalidCapability {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.0)
    }
}

impl std::error::Error for InvalidCapability {}

macro_rules! operations {
    ($name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(#[serde(rename = $wire)] $variant),+
        }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),+ }
            }
        }
    };
}

operations!(MetadataOperation {
    Search => "search", Detail => "detail", ListChapters => "list_chapters",
    ChapterDetail => "chapter_detail", ResolveAudio => "resolve_audio",
    FetchCover => "fetch_cover",
});
operations!(StoreOperation { ListPlugins => "list_plugins", GetPlugin => "get_plugin" });
operations!(ContentOperation {
    Probe => "probe", Open => "open", Close => "close", Cancel => "cancel",
    ExtractMetadata => "extract_metadata", ReadText => "read_text",
    RenderPage => "render_page", Seek => "seek",
});
operations!(ToolEntryPoint { InvokeTool => "invokeTool" });
operations!(HttpMethod {
    Get => "GET", Head => "HEAD", Post => "POST", Put => "PUT",
    Patch => "PATCH", Delete => "DELETE", Options => "OPTIONS",
});
operations!(RouteAuth {
    User => "user", Public => "public", Signed => "signed", PublicOrSigned => "public_or_signed",
});
operations!(UiSlot {
    SidebarPage => "app.sidebar_page", FloatingAction => "global.floating_action",
    Panel => "global.panel", BookAction => "book.detail_action",
});
operations!(UiContext { Global => "global", Book => "book", Reader => "reader" });
operations!(UiRenderMode { WebContainer => "web_container", Action => "action" });
operations!(SearchFieldType { Text => "text", Number => "number", Boolean => "boolean" });

impl RouteAuth {
    pub fn can_use_public_prefix(self) -> bool {
        !matches!(self, Self::User)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataProviderCap {
    pub id: String,
    pub operations: Vec<MetadataOperation>,
    #[serde(default)]
    pub auto_scrape: bool,
    #[serde(default)]
    pub aggregate_auto_scrape: bool,
    pub search_fields: Vec<SearchField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filters_schema: Option<Value>,
    pub result_fields: Vec<ResultField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchField {
    pub key: String,
    pub label: LocalizedText,
    pub required: bool,
    #[serde(rename = "type")]
    pub field_type: SearchFieldType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<LocalizedText>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultField {
    pub key: String,
    pub label: LocalizedText,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolProviderCap {
    pub id: String,
    pub invoke: ToolEntryPoint,
    pub tools: Vec<ToolDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDeclaration {
    pub name: String,
    pub description: LocalizedText,
    pub input_schema: Value,
    pub output_schema: Value,
    pub side_effects: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpRouteCap {
    pub id: String,
    pub route: RouteDeclaration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteDeclaration {
    pub method: HttpMethod,
    pub path: String,
    pub auth: RouteAuth,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiExtensionCap {
    pub id: String,
    pub slots: Vec<UiSlot>,
    pub contexts: Vec<UiContext>,
    pub title: LocalizedText,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub priority: i32,
    pub render: UiRenderConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiRenderConfig {
    pub mode: UiRenderMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<String>,
    pub bridge: UiBridge,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiBridge {
    pub capabilities: Vec<String>,
    pub host_methods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginStoreCap {
    pub id: String,
    pub operations: Vec<StoreOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentProcessorCap {
    pub id: String,
    pub extensions: Vec<String>,
    pub operations: Vec<ContentOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskHandlerCap {
    pub id: String,
    pub tasks: Vec<TaskDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskDeclaration {
    pub task_type: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub idempotent: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventHandlerCap {
    pub id: String,
    pub events: Vec<EventDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDeclaration {
    pub name: String,
    pub schema: Value,
}

impl Capability {
    /// Function names that a runtime adapter must find before registration.
    /// This list is generated from the declaration, not from free-form
    /// manifest `invoke` strings.
    pub fn required_exports(&self) -> Vec<&'static str> {
        match self {
            Self::MetadataProvider(cap) => cap.operations.iter().map(|op| op.as_str()).collect(),
            Self::FormatHandler(cap) => cap.operations.iter().map(|op| op.as_str()).collect(),
            Self::ToolProvider(_) => vec!["invokeTool"],
            Self::HttpRoute(_) | Self::EventHandler(_) => vec!["handle"],
            Self::UiExtension(_) => vec!["open"],
            Self::PluginStore(cap) => cap.operations.iter().map(|op| op.as_str()).collect(),
            Self::ContentProcessor(cap) => cap.operations.iter().map(|op| op.as_str()).collect(),
            Self::TaskHandler(_) => vec!["run"],
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::MetadataProvider(cap) => &cap.id,
            Self::FormatHandler(cap) => &cap.id,
            Self::ToolProvider(cap) => &cap.id,
            Self::HttpRoute(cap) => &cap.id,
            Self::UiExtension(cap) => &cap.id,
            Self::PluginStore(cap) => &cap.id,
            Self::ContentProcessor(cap) => &cap.id,
            Self::TaskHandler(cap) => &cap.id,
            Self::EventHandler(cap) => &cap.id,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::MetadataProvider(_) => "metadata_provider",
            Self::FormatHandler(_) => "format_handler",
            Self::ToolProvider(_) => "tool_provider",
            Self::HttpRoute(_) => "http_route",
            Self::UiExtension(_) => "ui_extension",
            Self::PluginStore(_) => "plugin_store",
            Self::ContentProcessor(_) => "content_processor",
            Self::TaskHandler(_) => "task_handler",
            Self::EventHandler(_) => "event_handler",
        }
    }

    pub fn supports(&self, operation: &str) -> bool {
        match self {
            Self::MetadataProvider(cap) => cap.operations.iter().any(|op| op.as_str() == operation),
            Self::FormatHandler(cap) => cap.operations.iter().any(|op| op.as_str() == operation),
            Self::ToolProvider(_) => operation == "invokeTool",
            Self::HttpRoute(_) | Self::EventHandler(_) => operation == "handle",
            Self::UiExtension(_) => operation == "open",
            Self::PluginStore(cap) => cap.operations.iter().any(|op| op.as_str() == operation),
            Self::ContentProcessor(cap) => cap.operations.iter().any(|op| op.as_str() == operation),
            Self::TaskHandler(_) => operation == "run",
        }
    }

    pub fn extensions(&self) -> &[String] {
        match self {
            Self::FormatHandler(cap) => &cap.extensions,
            Self::ContentProcessor(cap) => &cap.extensions,
            _ => &[],
        }
    }

    pub fn ui_bridge(&self) -> Option<&UiBridge> {
        match self {
            Self::UiExtension(cap) => Some(&cap.render.bridge),
            _ => None,
        }
    }

    pub fn validate(&self) -> Result<(), InvalidCapability> {
        safe_id(self.id(), "id")?;
        match self {
            Self::FormatHandler(cap) => cap.validate().map_err(|e| InvalidCapability(e.0)),
            Self::MetadataProvider(cap) => {
                unique_nonempty(&cap.operations, "operations")?;
                require(
                    cap.operations.contains(&MetadataOperation::Search),
                    "search is required",
                )?;
                require(
                    !cap.search_fields.is_empty(),
                    "search_fields must not be empty",
                )?;
                unique_names(
                    cap.search_fields.iter().map(|field| field.key.as_str()),
                    "search_fields",
                )?;
                for field in &cap.search_fields {
                    localized(&field.label, "search_fields.label")?;
                    if let Some(placeholder) = &field.placeholder {
                        localized(placeholder, "search_fields.placeholder")?;
                    }
                }
                let custom_fields: Vec<&str> = cap.search_fields.iter()
                    .map(|field| field.key.as_str())
                    .filter(|key| !matches!(*key, "title" | "author" | "narrator"))
                    .collect();
                if let Some(schema) = &cap.filters_schema {
                    validate_schema(schema)?;
                    require(schema.get("type").and_then(Value::as_str) == Some("object"),
                        "filters_schema must describe an object")?;
                    let properties = schema.get("properties").and_then(Value::as_object)
                        .ok_or_else(|| InvalidCapability("filters_schema needs properties".into()))?;
                    require(properties.keys().all(|key| custom_fields.contains(&key.as_str())),
                        "filters_schema properties must be declared search fields")?;
                    require(custom_fields.iter().all(|key| properties.contains_key(*key)),
                        "custom search fields must have a filters_schema property")?;
                } else {
                    require(custom_fields.is_empty(),
                        "custom search fields require filters_schema")?;
                }
                if cap.auto_scrape {
                    require(
                        cap.search_fields
                            .iter()
                            .any(|f| f.key == "title" && f.required),
                        "auto_scrape requires a required title field",
                    )?;
                }
                require(
                    !cap.aggregate_auto_scrape || cap.auto_scrape,
                    "aggregate_auto_scrape requires auto_scrape",
                )?;
                unique_names(
                    cap.result_fields.iter().map(|field| field.key.as_str()),
                    "result_fields",
                )?;
                for field in &cap.result_fields {
                    localized(&field.label, "result_fields.label")?;
                }
                Ok(())
            }
            Self::ToolProvider(cap) => {
                unique_names(cap.tools.iter().map(|tool| tool.name.as_str()), "tools")?;
                for tool in &cap.tools {
                    localized(&tool.description, "tools.description")?;
                    validate_schema(&tool.input_schema)?;
                    validate_schema(&tool.output_schema)?;
                }
                Ok(())
            }
            Self::HttpRoute(cap) => validate_route_path(&cap.route.path),
            Self::UiExtension(cap) => {
                unique_nonempty(&cap.slots, "slots")?;
                unique_nonempty(&cap.contexts, "contexts")?;
                localized(&cap.title, "title")?;
                match (&cap.render.mode, &cap.render.entry) {
                    (UiRenderMode::WebContainer, Some(entry)) => validate_ui_entry(entry)?,
                    (UiRenderMode::Action, None) => {}
                    _ => {
                        return Err(InvalidCapability(
                            "web_container requires entry; action must not declare entry".into(),
                        ));
                    }
                }
                unique_names_allow_empty(
                    cap.render.bridge.capabilities.iter().map(String::as_str),
                    "bridge.capabilities",
                )?;
                unique_names_allow_empty(
                    cap.render.bridge.host_methods.iter().map(String::as_str),
                    "bridge.host_methods",
                )
            }
            Self::PluginStore(cap) => {
                unique_nonempty(&cap.operations, "operations")?;
                require(
                    cap.operations.contains(&StoreOperation::ListPlugins),
                    "list_plugins is required",
                )
            }
            Self::ContentProcessor(cap) => {
                validate_extensions(&cap.extensions)?;
                unique_nonempty(&cap.operations, "operations")?;
                for op in [
                    ContentOperation::Probe,
                    ContentOperation::Open,
                    ContentOperation::Close,
                    ContentOperation::Cancel,
                ] {
                    require(
                        cap.operations.contains(&op),
                        "content_processor requires probe, open, close and cancel",
                    )?;
                }
                Ok(())
            }
            Self::TaskHandler(cap) => {
                unique_names(
                    cap.tasks.iter().map(|task| task.task_type.as_str()),
                    "tasks",
                )?;
                for task in &cap.tasks {
                    require(
                        !matches!(task.task_type.as_str(), "library_scan" | "write_metadata"),
                        "core task types are reserved",
                    )?;
                    validate_schema(&task.input_schema)?;
                    validate_schema(&task.output_schema)?;
                }
                Ok(())
            }
            Self::EventHandler(cap) => {
                unique_names(cap.events.iter().map(|event| event.name.as_str()), "events")?;
                for event in &cap.events {
                    validate_schema(&event.schema)?;
                }
                Ok(())
            }
        }
    }
}

/// Validate declarations together; references and namespace conflicts cannot
/// be checked by deserializing one capability in isolation.
pub fn validate_capabilities(capabilities: &[Capability]) -> Result<(), InvalidCapability> {
    unique_names(capabilities.iter().map(Capability::id), "capabilities")?;
    let mut tools = HashSet::new();
    let mut tasks = HashSet::new();
    let mut routes: Vec<&RouteDeclaration> = Vec::new();
    for capability in capabilities {
        capability.validate().map_err(|error| {
            InvalidCapability(format!("capabilities.{}: {}", capability.id(), error))
        })?;
        match capability {
            Capability::ToolProvider(cap) => {
                for tool in &cap.tools {
                    require(
                        tools.insert(tool.name.as_str()),
                        "duplicate tool name within plugin",
                    )?;
                }
            }
            Capability::TaskHandler(cap) => {
                for task in &cap.tasks {
                    require(
                        tasks.insert(task.task_type.as_str()),
                        "duplicate task type within plugin",
                    )?;
                }
            }
            Capability::HttpRoute(cap) => {
                require(
                    !routes.iter().any(|route| routes_overlap(route, &cap.route)),
                    "overlapping routes for the same method within plugin",
                )?;
                routes.push(&cap.route);
            }
            Capability::UiExtension(cap) => {
                for target in &cap.render.bridge.capabilities {
                    require(
                        capabilities
                            .iter()
                            .any(|capability| capability.id() == target),
                        &format!("bridge references undeclared capability: {target}"),
                    )?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Only complete `{parameter}` segments are dynamic. There are no colon,
/// wildcard or suffix aliases. Resource extensions belong in a separate segment.
pub fn validate_route_path(path: &str) -> Result<(), InvalidCapability> {
    require(
        path.starts_with('/') && path.len() <= 512 && (path == "/" || !path.ends_with('/')),
        "route.path must be an absolute canonical path (<= 512 bytes)",
    )?;
    if path == "/" {
        return Ok(());
    }
    let mut params = HashSet::new();
    for segment in path[1..].split('/') {
        if let Some(name) = route_parameter(segment) {
            safe_id(name, "route parameter")?;
            require(params.insert(name), "duplicate route parameter")?;
        } else {
            require(
                !segment.is_empty()
                    && !matches!(segment, "." | "..")
                    && segment
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')),
                "route.path permits literal safe segments or complete {parameter} segments",
            )?;
        }
    }
    Ok(())
}

pub fn route_parameter(segment: &str) -> Option<&str> {
    segment.strip_prefix('{')?.strip_suffix('}')
}

pub fn routes_overlap(left: &RouteDeclaration, right: &RouteDeclaration) -> bool {
    if left.method != right.method {
        return false;
    }
    let left: Vec<_> = left.path.split('/').collect();
    let right: Vec<_> = right.path.split('/').collect();
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| a == &b || route_parameter(a).is_some() || route_parameter(b).is_some())
}

/// Bound schema work before any runtime validator sees it. `$ref` may only
/// point within this schema; no file, network or package resolver is implicit.
/// The host additionally compiles schemas and validates payloads.
pub fn validate_schema(schema: &Value) -> Result<(), InvalidCapability> {
    require(schema.is_object(), "schema must be a JSON object")?;
    if let Some(dialect) = schema.get("$schema") {
        require(
            dialect.as_str() == Some("https://json-schema.org/draft/2020-12/schema"),
            "schema dialect must be 2020-12",
        )?;
    }
    require(
        schema.to_string().len() <= 64 * 1024,
        "schema exceeds 64 KiB",
    )?;
    let mut nodes = 0;
    fn walk(
        value: &Value,
        root: &Value,
        depth: usize,
        nodes: &mut usize,
        refs: &mut HashSet<String>,
    ) -> Result<(), InvalidCapability> {
        *nodes += 1;
        require(
            depth <= 32 && *nodes <= 4096,
            "schema exceeds depth/node budget",
        )?;
        match value {
            Value::Object(object) => {
                if let Some(reference) = object.get("$ref") {
                    let reference = reference
                        .as_str()
                        .ok_or_else(|| InvalidCapability("$ref must be a string".into()))?;
                    let pointer = reference.strip_prefix("#/").ok_or_else(|| {
                        InvalidCapability("$ref must be a local JSON pointer".into())
                    })?;
                    let target = root
                        .pointer(&format!("/{pointer}"))
                        .ok_or_else(|| InvalidCapability("unresolved local $ref".into()))?;
                    require(
                        refs.insert(reference.to_string()),
                        "cyclic schema reference",
                    )?;
                    walk(target, root, depth + 1, nodes, refs)?;
                    refs.remove(reference);
                }
                require(
                    !object.contains_key("$id") && !object.contains_key("$dynamicRef"),
                    "$id and $dynamicRef are not supported",
                )?;
                for child in object.values() {
                    walk(child, root, depth + 1, nodes, refs)?;
                }
            }
            Value::Array(values) => {
                for child in values {
                    walk(child, root, depth + 1, nodes, refs)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    walk(schema, schema, 0, &mut nodes, &mut HashSet::new())
}

fn safe_id(value: &str, field: &str) -> Result<(), InvalidCapability> {
    require(
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            && !value.contains(".."),
        &format!("{field} must be a safe nonempty identifier (<= 64 bytes)"),
    )
}

fn require(condition: bool, message: &str) -> Result<(), InvalidCapability> {
    if condition {
        Ok(())
    } else {
        Err(InvalidCapability(message.into()))
    }
}

fn localized(value: &LocalizedText, field: &str) -> Result<(), InvalidCapability> {
    require(
        !value.is_empty()
            && value.len() <= 16
            && value.iter().all(|(locale, text)| {
                !locale.is_empty()
                    && locale.len() <= 32
                    && locale
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    && !text.trim().is_empty()
                    && text.len() <= 4096
            }),
        &format!("{field} must be a nonempty locale-to-text mapping"),
    )
}

fn unique_nonempty<T: Eq + std::hash::Hash>(
    values: &[T],
    field: &str,
) -> Result<(), InvalidCapability> {
    require(
        !values.is_empty() && values.len() <= 128,
        &format!("{field} must contain 1..128 entries"),
    )?;
    let unique: HashSet<_> = values.iter().collect();
    require(
        unique.len() == values.len(),
        &format!("duplicate {field} entry"),
    )
}

fn unique_names<'a>(
    values: impl Iterator<Item = &'a str>,
    field: &str,
) -> Result<(), InvalidCapability> {
    let values: Vec<_> = values.collect();
    unique_nonempty(&values, field)?;
    for value in values {
        safe_id(value, field)?;
    }
    Ok(())
}

fn unique_names_allow_empty<'a>(
    values: impl Iterator<Item = &'a str>,
    field: &str,
) -> Result<(), InvalidCapability> {
    let values: Vec<_> = values.collect();
    if values.is_empty() {
        return Ok(());
    }
    unique_names(values.into_iter(), field)
}

fn validate_extensions(extensions: &[String]) -> Result<(), InvalidCapability> {
    unique_nonempty(extensions, "extensions")?;
    for extension in extensions {
        require(
            extension.len() <= 16
                && extension
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()),
            "extensions must be lowercase without dots or wildcards",
        )?;
    }
    Ok(())
}

fn validate_ui_entry(entry: &str) -> Result<(), InvalidCapability> {
    require(
        entry.starts_with("ui/")
            && entry.ends_with(".html")
            && entry.len() <= 240
            && entry.split('/').all(|s| {
                !s.is_empty()
                    && !matches!(s, "." | "..")
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            }),
        "web_container entry must be a safe package-local ui/*.html path",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn all_nine_capabilities_round_trip_with_declared_operations() {
        let schema = json!({"type":"object","additionalProperties":false});
        let caps: Vec<Capability> = serde_json::from_value(json!([
            {"id":"metadata","kind":"metadata_provider","operations":["search"],
             "search_fields":[{"key":"title","label":{"en":"Title"},"required":true,"type":"text"}],
             "result_fields":[{"key":"title","label":{"en":"Title"}}]},
            {"id":"format","kind":"format_handler","extensions":["special"],"operations":["probe","extract_metadata"]},
            {"id":"tools","kind":"tool_provider","invoke":"invokeTool","tools":[
                {"name":"books.search","description":{"en":"Search"},"input_schema":schema,"output_schema":schema,"side_effects":false}]},
            {"id":"route","kind":"http_route","route":{"method":"GET","path":"/feed/{book_id}","auth":"signed"}},
            {"id":"ui","kind":"ui_extension","slots":["global.panel"],"contexts":["global"],
             "title":{"en":"Tools"},"render":{"mode":"web_container","entry":"ui/tools.html",
             "bridge":{"capabilities":["tools"],"host_methods":["books.list"]}}},
            {"id":"store","kind":"plugin_store","operations":["list_plugins"]},
            {"id":"content","kind":"content_processor","extensions":["txt"],"operations":["probe","open","close","cancel","read_text"]},
            {"id":"task","kind":"task_handler","tasks":[{"task_type":"books.summarize","input_schema":schema,"output_schema":schema,"idempotent":true}]},
            {"id":"event","kind":"event_handler","events":[{"name":"book.added","schema":schema}]}
        ])).unwrap();
        validate_capabilities(&caps).unwrap();
        let wire = serde_json::to_value(&caps).unwrap();
        assert_eq!(
            serde_json::from_value::<Vec<Capability>>(wire).unwrap(),
            caps
        );
        for (cap, operation) in caps.iter().zip([
            "search",
            "extract_metadata",
            "invokeTool",
            "handle",
            "open",
            "list_plugins",
            "read_text",
            "run",
            "handle",
        ]) {
            assert!(cap.supports(operation));
            assert!(!cap.supports("execute"));
            assert!(cap.required_exports().contains(&operation));
        }
    }

    #[test]
    fn rejects_old_fields_unknown_kinds_and_arbitrary_entry_points() {
        for cap in [
            json!({"kind":"plugin_store","id":"store","operations":["list_plugins"],"invoke":"listPlugins"}),
            json!({"kind":"client_extension","id":"ui"}),
            json!({"kind":"tool_provider","id":"tools","invoke":"execute","tools":[]}),
            json!({"kind":"content_processor","id":"content","matches":{"extensions":["txt"]},"operations":["probe"]}),
            json!({"kind":"format_handler","id":"format","extensions":["xm"],"operations":["probe","decode"]}),
            json!({"kind":"http_route","id":"route","route":{"method":"*","path":"/x","auth":"public"}}),
            json!({"kind":"http_route","id":"route","route":{"method":"GET","path":"/x","auth":"publci"}}),
        ] {
            assert!(serde_json::from_value::<Capability>(cap).is_err());
        }
    }

    #[test]
    fn rejects_missing_lifecycle_duplicate_ids_and_dangling_bridges() {
        let mut cap: Capability = serde_json::from_value(json!({
            "id":"content","kind":"content_processor","extensions":["txt"],"operations":["probe","read_text"]
        })).unwrap();
        assert!(cap.validate().is_err());
        if let Capability::ContentProcessor(inner) = &mut cap {
            inner.operations.extend([
                ContentOperation::Open,
                ContentOperation::Close,
                ContentOperation::Cancel,
            ]);
        }
        cap.validate().unwrap();
        assert!(validate_capabilities(&[cap.clone(), cap]).is_err());
        let ui: Capability = serde_json::from_value(json!({
            "id":"ui","kind":"ui_extension","slots":["global.panel"],"contexts":["global"],
            "title":{"en":"Tools"},"render":{"mode":"action","bridge":{"capabilities":["missing"],"host_methods":[]}}
        })).unwrap();
        assert!(
            validate_capabilities(&[ui])
                .unwrap_err()
                .0
                .contains("missing")
        );
    }

    #[test]
    fn routes_have_one_syntax_and_reject_ambiguous_matches() {
        for path in [
            "/rss/:id.xml",
            "/rss/{id}.xml",
            "/rss/*",
            "/rss/{id}/{id}",
            "/rss//feed",
            "/rss/../feed",
            "rss/feed",
            "/rss/",
        ] {
            assert!(validate_route_path(path).is_err(), "{path}");
        }
        validate_route_path("/rss/{book_id}/feed.xml").unwrap();
        let route = |path: &str| RouteDeclaration {
            method: HttpMethod::Get,
            path: path.into(),
            auth: RouteAuth::User,
        };
        assert!(routes_overlap(&route("/rss/{id}"), &route("/rss/latest")));
        assert!(routes_overlap(
            &route("/rss/{id}"),
            &route("/rss/{book_id}")
        ));
        assert!(!routes_overlap(
            &route("/rss/{id}/feed"),
            &route("/cover/{id}/feed")
        ));
    }

    #[test]
    fn schemas_are_bounded_and_never_resolve_external_references() {
        for schema in [
            json!({"$ref":"https://example.com/schema.json"}),
            json!({"$ref":"file:///secret.json"}),
            json!({"$ref":"#/$defs/missing"}),
            json!({"$id":"https://example.com","type":"object"}),
            json!({"$schema":"http://json-schema.org/draft-07/schema#"}),
            Value::Bool(true),
            json!({"$defs":{"loop":{"$ref":"#/$defs/loop"}}}),
        ] {
            assert!(validate_schema(&schema).is_err());
        }
        validate_schema(&json!({"$defs":{"name":{"type":"string"}},"properties":{"name":{"$ref":"#/$defs/name"}}})).unwrap();
        let mut deep = json!({"type":"string"});
        for _ in 0..34 {
            deep = json!({"items":deep});
        }
        assert!(validate_schema(&deep).is_err());
    }
}
