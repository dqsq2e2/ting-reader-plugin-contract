//! Manifest identity, requested permissions and capability declarations.
//! Runtime specific package resource checks are performed after extraction.

use crate::capability::{Capability, InvalidCapability, LocalizedText, validate_capabilities};
use semver::Version;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: LocalizedText,
    pub runtime: Runtime,
    pub entry_point: String,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_schema: Option<Value>,
    pub min_core_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_flutter_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default)]
    pub admin_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    Wasm,
    Javascript,
    Native,
}

impl Runtime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wasm => "wasm",
            Self::Javascript => "javascript",
            Self::Native => "native",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub plugin_id: String,
    pub version_requirement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Permission {
    NetworkAccess {
        domain: String,
    },
    FileRead {
        path: String,
    },
    FileWrite {
        path: String,
    },
    EventSubscribe {
        event: String,
    },
    BooksRead,
    BooksWrite,
    LibrariesRead,
    ChaptersRead,
    ChaptersWrite,
    LibrariesWrite,
    ProgressRead,
    MediaReadUrl,
    PluginRouteSign,
    MetadataWrite,
    TaskCreate,
    CacheRead,
    CacheWrite,
    PlaylistsRead,
    PlaylistsWrite,
    FavoritesRead,
    FavoritesWrite,
    UserSettingsRead,
    UserSettingsWrite,
    ConfigRead,
    StorageRead,
    StorageWrite,
    TaskRead,
    TaskManage,
    TaskProgress,
    HtmlParse,
    PluginRouteRevoke,
    EventPublish,
    CapabilityInvoke {
        plugin_id: String,
        capability_id: String,
    },
}

impl std::fmt::Display for Permission {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Keep the same machine-readable declaration in management responses.
        let declaration = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;
        formatter.write_str(&declaration)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PermissionFields {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default, deserialize_with = "present_value")]
    domain: Option<Value>,
    #[serde(default, deserialize_with = "present_value")]
    path: Option<Value>,
    #[serde(default, deserialize_with = "present_value")]
    event: Option<Value>,
    #[serde(default, deserialize_with = "present_value")]
    plugin_id: Option<Value>,
    #[serde(default, deserialize_with = "present_value")]
    capability_id: Option<Value>,
}

fn present_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

impl<'de> Deserialize<'de> for Permission {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = PermissionFields::deserialize(deserializer)?;
        let fields = [
            ("domain", raw.domain.clone()),
            ("path", raw.path.clone()),
            ("event", raw.event.clone()),
            ("plugin_id", raw.plugin_id.clone()),
            ("capability_id", raw.capability_id.clone()),
        ];
        let present: Vec<_> = fields
            .into_iter()
            .filter_map(|(name, value)| value.map(|v| (name, v)))
            .collect();
        let expected = match raw.kind.as_str() {
            "network_access" => Some("domain"),
            "file_read" | "file_write" => Some("path"),
            "event_subscribe" => Some("event"),
            "capability_invoke" => Some("plugin_id"),
            "books_read"
            | "books_write"
            | "libraries_read"
            | "chapters_read"
            | "chapters_write"
            | "libraries_write"
            | "progress_read"
            | "media_read_url"
            | "plugin_route_sign"
            | "metadata_write"
            | "task_create"
            | "cache_read"
            | "cache_write"
            | "playlists_read"
            | "playlists_write"
            | "favorites_read"
            | "favorites_write"
            | "user_settings_read"
            | "user_settings_write"
            | "config_read"
            | "storage_read"
            | "storage_write"
            | "task_read"
            | "task_manage"
            | "task_progress"
            | "html_parse"
            | "plugin_route_revoke"
            | "event_publish" => None,
            _ => return Err(D::Error::custom("unknown permission type")),
        };
        if raw.kind == "capability_invoke" {
            let plugin_id = raw
                .plugin_id
                .as_ref()
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| D::Error::custom("capability_invoke.plugin_id is required"))?;
            let capability_id = raw
                .capability_id
                .as_ref()
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| D::Error::custom("capability_invoke.capability_id is required"))?;
            if present.len() != 2 {
                return Err(D::Error::custom(
                    "capability_invoke requires plugin_id and capability_id only",
                ));
            }
            return Ok(Self::CapabilityInvoke {
                plugin_id: plugin_id.into(),
                capability_id: capability_id.into(),
            });
        }
        let value = match (expected, present.as_slice()) {
            (Some(expected), [(actual, value)]) if *actual == expected => value
                .as_str()
                .ok_or_else(|| D::Error::custom("permission scope must be a string"))?,
            (None, []) => "",
            _ => {
                return Err(D::Error::custom(
                    "unexpected, missing or duplicate permission scope",
                ));
            }
        };
        Ok(match raw.kind.as_str() {
            "network_access" => Self::NetworkAccess {
                domain: value.into(),
            },
            "file_read" => Self::FileRead { path: value.into() },
            "file_write" => Self::FileWrite { path: value.into() },
            "event_subscribe" => Self::EventSubscribe {
                event: value.into(),
            },
            "books_read" => Self::BooksRead,
            "books_write" => Self::BooksWrite,
            "libraries_read" => Self::LibrariesRead,
            "chapters_read" => Self::ChaptersRead,
            "chapters_write" => Self::ChaptersWrite,
            "libraries_write" => Self::LibrariesWrite,
            "progress_read" => Self::ProgressRead,
            "media_read_url" => Self::MediaReadUrl,
            "plugin_route_sign" => Self::PluginRouteSign,
            "metadata_write" => Self::MetadataWrite,
            "task_create" => Self::TaskCreate,
            "cache_read" => Self::CacheRead,
            "cache_write" => Self::CacheWrite,
            "playlists_read" => Self::PlaylistsRead,
            "playlists_write" => Self::PlaylistsWrite,
            "favorites_read" => Self::FavoritesRead,
            "favorites_write" => Self::FavoritesWrite,
            "user_settings_read" => Self::UserSettingsRead,
            "user_settings_write" => Self::UserSettingsWrite,
            "config_read" => Self::ConfigRead,
            "storage_read" => Self::StorageRead,
            "storage_write" => Self::StorageWrite,
            "task_read" => Self::TaskRead,
            "task_manage" => Self::TaskManage,
            "task_progress" => Self::TaskProgress,
            "html_parse" => Self::HtmlParse,
            "plugin_route_revoke" => Self::PluginRouteRevoke,
            "event_publish" => Self::EventPublish,
            _ => return Err(D::Error::custom("unknown permission type")),
        })
    }
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), InvalidCapability> {
        let safe_edge = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let id_bytes = self.id.as_bytes();
        require(
            !id_bytes.is_empty()
                && id_bytes.len() <= 64
                && safe_edge(id_bytes[0])
                && safe_edge(id_bytes[id_bytes.len() - 1])
                && !self.id.contains("..")
                && id_bytes
                    .iter()
                    .all(|b| safe_edge(*b) || matches!(*b, b'-' | b'_' | b'.'))
                && !reserved_windows_name(&self.id),
            "id must be a safe lowercase plugin identifier (<= 64 bytes)",
        )?;
        require(
            !self.name.is_empty()
                && self.name == self.name.trim()
                && self.name.chars().count() <= 128
                && !self
                    .name
                    .chars()
                    .any(|ch| ch.is_control() || r#"<>:\"/\\|?*"#.contains(ch))
                && !self.name.ends_with([' ', '.'])
                && !reserved_windows_name(&self.name),
            "name must be a safe nonempty plugin name (<= 128 characters)",
        )?;
        require(
            self.version.len() <= 64 && Version::parse(&self.version).is_ok(),
            "version must be valid SemVer (<= 64 bytes)",
        )?;
        let minimum = Version::parse(&self.min_core_version)
            .map_err(|_| InvalidCapability("min_core_version must be valid SemVer".into()))?;
        require(
            minimum >= Version::parse(crate::MIN_PLUGIN_CORE_VERSION).expect("valid core version"),
            "min_core_version is below the 2.0 plugin contract",
        )?;
        if let Some(version) = &self.min_flutter_version {
            require(
                Version::parse(version).is_ok(),
                "min_flutter_version must be valid SemVer",
            )?;
        }
        require(
            self.entry_point.len() <= 240
                && !self.entry_point.contains('\\')
                && self.entry_point.split('/').all(|part| {
                    !part.is_empty()
                        && !matches!(part, "." | "..")
                        && !part.ends_with([' ', '.'])
                        && !reserved_windows_name(part)
                        && !part
                            .chars()
                            .any(|ch| ch.is_control() || r#"<>:\"/\\|?*"#.contains(ch))
                }),
            "entry_point must be a safe relative package path (<= 240 bytes)",
        )?;
        validate_capabilities(&self.capabilities)?;
        require(
            !self.description.is_empty()
                && self.description.len() <= 16
                && self.description.iter().all(|(lang, text)| {
                    !lang.is_empty()
                        && lang.len() <= 32
                        && !text.trim().is_empty()
                        && text.len() <= 4096
                }),
            "description must be a locale-to-text mapping",
        )?;
        let extension = self.entry_point.rsplit('.').next();
        require(
            match self.runtime {
                Runtime::Wasm => extension == Some("wasm"),
                Runtime::Javascript => extension == Some("js"),
                Runtime::Native => matches!(extension, Some("dll" | "so" | "dylib")),
            },
            "runtime must match the declared entry_point extension",
        )?;
        let mut permissions = HashSet::new();
        for permission in &self.permissions {
            // Same grant requested twice cannot expand effective permissions.
            let key = serde_json::to_string(permission)
                .map_err(|_| InvalidCapability("invalid permission".into()))?;
            require(permissions.insert(key), "duplicate permission")?;
            match permission {
                Permission::NetworkAccess { domain } => {
                    require(
                        !domain.is_empty()
                            && domain.len() <= 255
                            && !domain.contains('/')
                            && !domain.chars().any(char::is_whitespace),
                        "network_access.domain must be a bounded domain or *",
                    )?;
                }
                Permission::FileRead { path } | Permission::FileWrite { path } => {
                    require(
                        !path.is_empty() && path.len() <= 240 && !path.contains('\0'),
                        "file permission path must be nonempty and bounded",
                    )?;
                }
                Permission::EventSubscribe { event } => {
                    require(
                        !event.is_empty() && event.len() <= 128,
                        "event_subscribe.event must be nonempty and bounded",
                    )?;
                }
                Permission::CapabilityInvoke {
                    plugin_id,
                    capability_id,
                } => {
                    require(
                        !plugin_id.is_empty()
                            && plugin_id.len() <= 64
                            && plugin_id.bytes().all(|byte| {
                                byte.is_ascii_lowercase()
                                    || byte.is_ascii_digit()
                                    || matches!(byte, b'-' | b'_' | b'.' | b'@')
                            }),
                        "capability_invoke.plugin_id must be a safe plugin instance ID",
                    )?;
                    require(
                        !capability_id.is_empty()
                            && capability_id.len() <= 128
                            && capability_id.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
                            }),
                        "capability_invoke.capability_id must be a safe capability ID",
                    )?;
                }
                _ => {}
            }
        }
        let mut dependencies = HashSet::new();
        for dependency in &self.dependencies {
            require(
                !dependency.plugin_id.is_empty()
                    && !dependency.version_requirement.is_empty()
                    && dependencies.insert(dependency.plugin_id.as_str()),
                "dependencies require unique plugin_id and version_requirement",
            )?;
        }
        if let Some(schema) = &self.config_schema {
            crate::capability::validate_schema(schema)?;
        }
        Ok(())
    }
}

fn reserved_windows_name(value: &str) -> bool {
    let stem = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

fn require(condition: bool, message: &str) -> Result<(), InvalidCapability> {
    if condition {
        Ok(())
    } else {
        Err(InvalidCapability(message.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "id":"store","name":"Store","version":"2.0.0",
            "author":"Ting Reader","description":{"en":"Official Store"},
            "runtime":"javascript","entry_point":"plugin.js",
            "min_core_version":"2.0.0",
            "capabilities":[{"id":"store","kind":"plugin_store","operations":["list_plugins"]}],
            "permissions":[{"type":"network_access","domain":"example.org"}],
            "dependencies":[]
        })
    }

    #[test]
    fn requires_exact_top_level_and_permission_fields() {
        serde_json::from_value::<PluginManifest>(valid())
            .unwrap()
            .validate()
            .unwrap();
        for (field, value) in [
            ("unknown", json!(true)),
            ("npm_dependencies", json!([{"name":"hello"}])),
            ("description_en", json!("old")),
            ("plugin_type", json!("scraper")),
            ("supported_extensions", json!(["xm"])),
            ("runtime", json!("python")),
        ] {
            let mut manifest = valid();
            manifest[field] = value.clone();
            assert!(
                serde_json::from_value::<PluginManifest>(manifest).is_err(),
                "{field}"
            );
        }
        for permission in [
            json!({"type":"network_access","value":"example.org"}),
            json!({"type":"books_read","value":"ignored"}),
            json!({"type":"file_read"}),
            json!({"type":"unknown"}),
        ] {
            let mut manifest = valid();
            manifest["permissions"] = json!([permission]);
            assert!(
                serde_json::from_value::<PluginManifest>(manifest).is_err(),
                "{permission}"
            );
        }
    }

    #[test]
    fn checks_runtime_permissions_and_schema_references() {
        let mut manifest: PluginManifest = serde_json::from_value(valid()).unwrap();
        manifest.runtime = Runtime::Native;
        assert!(manifest.validate().is_err());
        manifest.runtime = Runtime::Javascript;
        manifest.permissions.push(Permission::NetworkAccess {
            domain: "example.org".into(),
        });
        assert!(manifest.validate().is_err());
        manifest.permissions.pop();
        manifest.config_schema = Some(json!({"$ref":"https://example.org/schema.json"}));
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn rejects_invalid_identity_versions_and_entry_paths_at_packaging_boundary() {
        for (field, value) in [
            ("id", json!("CON")),
            ("id", json!("sample/../other")),
            ("name", json!("Bad/Name")),
            ("version", json!("v2.0")),
            ("min_core_version", json!("1.9.0")),
            ("min_core_version", json!("v2.0.0")),
            ("entry_point", json!("../other/plugin.js")),
            ("entry_point", json!("ui\\plugin.js")),
        ] {
            let mut manifest = valid();
            manifest[field] = value.clone();
            assert!(
                serde_json::from_value::<PluginManifest>(manifest)
                    .unwrap()
                    .validate()
                    .is_err(),
                "{field}: {value}"
            );
        }
    }
}
