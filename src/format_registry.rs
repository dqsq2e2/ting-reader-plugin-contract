//! Extension-based format candidate selection. The caller probes each
//! candidate using the Host resource manager before choosing a provider.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    format::{FormatHandlerCap, InvalidFormatCapability},
    format_calls::ProbeResult,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FormatProviderKey {
    pub plugin_id: String,
    pub capability_id: String,
}

#[derive(Debug, Default)]
pub struct FormatRegistry {
    providers: BTreeMap<FormatProviderKey, FormatHandlerCap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatSelectionError {
    Invalid(String),
    Conflict(Vec<FormatProviderKey>),
}

impl std::fmt::Display for FormatSelectionError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => fmt.write_str(message),
            Self::Conflict(providers) => {
                write!(fmt, "multiple format providers matched: {providers:?}")
            }
        }
    }
}

impl std::error::Error for FormatSelectionError {}

impl From<InvalidFormatCapability> for FormatSelectionError {
    fn from(error: InvalidFormatCapability) -> Self {
        Self::Invalid(error.0)
    }
}

impl FormatRegistry {
    pub fn register(
        &mut self,
        plugin_id: &str,
        declaration: FormatHandlerCap,
    ) -> Result<FormatProviderKey, FormatSelectionError> {
        declaration.validate()?;
        if plugin_id.is_empty() || plugin_id.len() > 128 {
            return Err(FormatSelectionError::Invalid(
                "plugin ID must be nonempty and bounded".into(),
            ));
        }
        let key = FormatProviderKey {
            plugin_id: plugin_id.into(),
            capability_id: declaration.id.clone(),
        };
        if self.providers.contains_key(&key) {
            return Err(FormatSelectionError::Invalid(format!(
                "duplicate format provider: {key:?}"
            )));
        }
        self.providers.insert(key.clone(), declaration);
        Ok(key)
    }

    pub fn remove(&mut self, key: &FormatProviderKey) {
        self.providers.remove(key);
    }

    /// Extension narrows candidates. A match still requires `probe` to
    /// confirm the file contents. Extension names are normalized at the
    /// trusted caller boundary; invalid hints never select a provider.
    pub fn candidates(&self, extension: &str) -> Vec<FormatProviderKey> {
        let hint = extension
            .strip_prefix('.')
            .unwrap_or(extension)
            .to_ascii_lowercase();
        if hint.is_empty()
            || hint.len() > 16
            || !hint
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        {
            return Vec::new();
        }
        self.providers
            .iter()
            .filter(|(_, cap)| cap.extensions.iter().any(|ext| ext == &hint))
            .map(|(key, _)| key.clone())
            .collect()
    }

    /// Accept exactly one terminal result for each candidate. NeedMore is
    /// not terminal; the caller must perform a bounded additional probe or
    /// fail the selection. `binding` is an administrator-selected provider.
    pub fn select(
        &self,
        extension: &str,
        results: &[(FormatProviderKey, ProbeResult)],
        binding: Option<&FormatProviderKey>,
    ) -> Result<Option<FormatProviderKey>, FormatSelectionError> {
        let expected: BTreeSet<_> = self.candidates(extension).into_iter().collect();
        let mut seen = BTreeSet::new();
        let mut matched = Vec::new();
        for (key, result) in results {
            if !expected.contains(key) || !seen.insert(key.clone()) {
                return Err(FormatSelectionError::Invalid(
                    "unknown or duplicate format probe result".into(),
                ));
            }
            match result {
                ProbeResult::Match => matched.push(key.clone()),
                ProbeResult::NoMatch => {}
                ProbeResult::NeedMore { .. } => {
                    return Err(FormatSelectionError::Invalid(
                        "format probe still needs bytes".into(),
                    ));
                }
            }
        }
        if seen != expected {
            return Err(FormatSelectionError::Invalid(
                "missing format probe result".into(),
            ));
        }
        if let Some(binding) = binding {
            if matched.contains(binding) {
                return Ok(Some(binding.clone()));
            }
            return Err(FormatSelectionError::Invalid(
                "bound format provider did not match".into(),
            ));
        }
        match matched.len() {
            0 => Ok(None),
            1 => Ok(matched.pop()),
            _ => Err(FormatSelectionError::Conflict(matched)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::FormatOperation;

    fn metadata_cap(id: &str, ext: &str) -> FormatHandlerCap {
        FormatHandlerCap {
            id: id.into(),
            extensions: vec![ext.into()],
            operations: vec![FormatOperation::Probe, FormatOperation::ExtractMetadata],
        }
    }

    #[test]
    fn declaration_and_probe_select_without_hardcoded_format_logic() {
        let mut registry = FormatRegistry::default();
        let first = registry
            .register("example-a", metadata_cap("format", "abc"))
            .unwrap();
        let other = registry
            .register("example-b", metadata_cap("format", "abc"))
            .unwrap();
        assert_eq!(
            registry.candidates(".ABC"),
            vec![first.clone(), other.clone()]
        );
        assert!(registry.candidates("..abc").is_empty());
        let results = [
            (first.clone(), ProbeResult::Match),
            (other.clone(), ProbeResult::NoMatch),
        ];
        assert_eq!(
            registry.select("abc", &results, None).unwrap(),
            Some(first.clone())
        );
        assert!(registry.select("abc", &results, Some(&other)).is_err());
        registry.remove(&first);
        assert_eq!(registry.candidates("abc"), vec![other]);
    }

    #[test]
    fn overlapping_providers_require_explicit_matching_binding() {
        let mut registry = FormatRegistry::default();
        let one = registry
            .register("a", metadata_cap("format", "abc"))
            .unwrap();
        let two = registry
            .register("b", metadata_cap("format", "abc"))
            .unwrap();
        let results = [
            (one.clone(), ProbeResult::Match),
            (two.clone(), ProbeResult::Match),
        ];
        assert_eq!(
            registry.select("abc", &results, None),
            Err(FormatSelectionError::Conflict(vec![
                one.clone(),
                two.clone()
            ]))
        );
        assert_eq!(
            registry.select("abc", &results, Some(&two)).unwrap(),
            Some(two)
        );
        assert!(registry.select("abc", &results[..1], None).is_err());
        assert!(
            registry
                .select(
                    "abc",
                    &[
                        (one.clone(), ProbeResult::NeedMore { total_bytes: 8192 }),
                        (one, ProbeResult::Match)
                    ],
                    None
                )
                .is_err()
        );
    }

    #[test]
    fn refuses_duplicate_provider_and_invalid_declaration() {
        let mut registry = FormatRegistry::default();
        registry
            .register("a", metadata_cap("format", "abc"))
            .unwrap();
        assert!(
            registry
                .register("a", metadata_cap("format", "abc"))
                .is_err()
        );
        assert!(
            registry
                .register("b", metadata_cap("format", "../abc"))
                .is_err()
        );
    }
}
