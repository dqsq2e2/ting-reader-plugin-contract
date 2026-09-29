//! Search and optional media operations for metadata providers.
//! Unknown values are represented as null, never as invented zeros or IDs.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    #[serde(deserialize_with = "required_nullable")]
    pub title: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub author: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub narrator: Option<String>,
    pub page: u32,
    pub page_size: u32,
    #[serde(default)]
    pub filters: serde_json::Map<String, Value>,
    #[serde(default)]
    pub chapter_candidates: Vec<ChapterCandidate>,
    #[serde(default)]
    pub context: Option<SearchContext>,
}

/// Chapter titles supplied by the host for a single cleanup batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterCandidate {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    Search,
    Aggregate,
    TitleCleanup,
}

/// Optional host-provided context for aggregate metadata selection.
/// Platform-specific search parameters continue to live in `filters`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchContext {
    pub mode: SearchMode,
    #[serde(default)]
    pub candidates: Vec<Value>,
    #[serde(default)]
    pub merged_metadata: Option<Value>,
    #[serde(default)]
    pub scanner_context: Option<Value>,
    #[serde(default)]
    pub scraper_query: Option<String>,
}

impl SearchRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.page == 0 || !(1..=100).contains(&self.page_size) {
            return Err("Search page and page_size must be within bounds");
        }
        if [&self.title, &self.author, &self.narrator].into_iter()
            .flatten().any(|text| text.len() > 512)
        {
            return Err("Search field exceeds 512 bytes");
        }
        if serde_json::to_vec(&self.filters)
            .is_ok_and(|bytes| bytes.len() > 8 * 1024)
        {
            return Err("Search filters exceed 8 KiB");
        }
        if self.chapter_candidates.len() > 500
            || self.chapter_candidates.iter().any(|chapter| {
                chapter.id.trim().is_empty() || chapter.id.len() > 512
                    || chapter.title.trim().is_empty() || chapter.title.len() > 512
            })
        {
            return Err("Chapter candidates exceed bounds");
        }
        if let Some(context) = &self.context {
            if context.candidates.len() > 100
                || context.scraper_query.as_ref().is_some_and(|text| text.len() > 512)
                || serde_json::to_vec(context).is_ok_and(|bytes| bytes.len() > 128 * 1024)
            {
                return Err("Search context exceeds bounds");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchPage {
    pub items: Vec<ScraperResult>,
    pub page: u32,
    pub page_size: u32,
    #[serde(deserialize_with = "required_nullable")]
    pub total: Option<u64>,
    #[serde(deserialize_with = "required_nullable")]
    pub has_more: Option<bool>,
}

impl SearchPage {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.page == 0 || !(1..=100).contains(&self.page_size)
            || self.items.len() > self.page_size as usize
            || self.items.len() > 100
        {
            return Err("Search page exceeds declared bounds");
        }
        for item in &self.items {
            item.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScraperResult {
    #[serde(deserialize_with = "required_nullable")]
    pub id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub source_url: Option<String>,
    pub title: String,
    #[serde(deserialize_with = "required_nullable")]
    pub author: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub narrator: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub cover_url: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub intro: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub subtitle: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub publisher: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub language: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub genre: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub published_year: Option<u16>,
    #[serde(deserialize_with = "required_nullable")]
    pub published_date: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub isbn: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub asin: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub explicit: Option<bool>,
    #[serde(deserialize_with = "required_nullable")]
    pub abridged: Option<bool>,
    pub tags: Vec<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub duration: Option<u64>,
    #[serde(deserialize_with = "required_nullable")]
    pub score: Option<f64>,
    #[serde(deserialize_with = "required_nullable")]
    pub chapter_title_template: Option<String>,
    pub chapter_titles: Vec<String>,
}

impl ScraperResult {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.title.trim().is_empty() || self.title.len() > 512 {
            return Err("Result title must be nonempty and within 512 bytes");
        }
        if self.id.as_ref().is_some_and(|id| id.trim().is_empty())
            || self.source_url.as_ref().is_some_and(|url| url.trim().is_empty())
        {
            return Err("Empty source references must use null");
        }
        if self.score.is_some_and(|score| !score.is_finite() || !(0.0..=1.0).contains(&score)) {
            return Err("Match score must be finite and between zero and one");
        }
        if self.tags.len() > 64
            || self.tags.iter().any(|tag| tag.trim().is_empty() || tag.len() > 256)
        {
            return Err("Result tags exceed bounds");
        }
        if self.chapter_titles.len() > 500 || self.chapter_titles.iter()
            .any(|title| title.trim().is_empty() || title.len() > 512)
        {
            return Err("Chapter titles exceed bounds");
        }
        if self.published_date.as_ref().is_some_and(|date| {
            date.len() != 10 || date.as_bytes().get(4) != Some(&b'-')
                || date.as_bytes().get(7) != Some(&b'-')
                || !date.bytes().enumerate().all(|(i, byte)| i == 4 || i == 7 || byte.is_ascii_digit())
        }) {
            return Err("Published date must use YYYY-MM-DD");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    pub id: Option<String>,
    pub url: Option<String>,
}

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

impl SourceRef {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.as_deref().is_some_and(|id| id.trim().is_empty())
            || self.url.as_deref().is_some_and(|url| url.trim().is_empty())
            || self.id.is_none() && self.url.is_none()
        {
            return Err("Source reference requires a nonempty id or URL");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterListRequest {
    pub source: SourceRef,
    pub page: u32,
    pub page_size: u32,
}

impl ChapterListRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.source.validate()?;
        if self.page == 0 || !(1..=100).contains(&self.page_size) {
            return Err("Chapter page and page_size must be within bounds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterId {
    pub chapter_id: String,
}

impl ChapterId {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.chapter_id.trim().is_empty() || self.chapter_id.len() > 512 {
            return Err("Chapter ID must be nonempty and within 512 bytes");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterItem {
    pub id: String,
    pub title: String,
    pub index: u32,
    #[serde(deserialize_with = "required_nullable")]
    pub duration: Option<u64>,
    #[serde(deserialize_with = "required_nullable")]
    pub source_url: Option<String>,
}

impl ChapterItem {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.trim().is_empty() || self.id.len() > 512
            || self.title.trim().is_empty() || self.title.len() > 512
        {
            return Err("Chapter ID and title must be nonempty and within 512 bytes");
        }
        if self.source_url.as_ref().is_some_and(|url| url.trim().is_empty()) {
            return Err("Empty chapter URL must use null");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterPage {
    pub items: Vec<ChapterItem>,
    pub page: u32,
    pub page_size: u32,
    #[serde(deserialize_with = "required_nullable")]
    pub total: Option<u64>,
    #[serde(deserialize_with = "required_nullable")]
    pub has_more: Option<bool>,
}

impl ChapterPage {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.page == 0 || !(1..=100).contains(&self.page_size)
            || self.items.len() > self.page_size as usize
        {
            return Err("Chapter page exceeds declared bounds");
        }
        for chapter in &self.items {
            chapter.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChapterDetail {
    pub chapter: ChapterItem,
    #[serde(deserialize_with = "required_nullable")]
    pub description: Option<String>,
}

impl ChapterDetail {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.chapter.validate()?;
        if self.description.as_ref().is_some_and(|text| text.len() > 32 * 1024) {
            return Err("Chapter description exceeds 32 KiB");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSourceType {
    Direct,
    Hls,
    Proxy,
}

/// Credentials and origin headers stay in the Host, outside the plugin DTO.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaSourceDescriptor {
    #[serde(rename = "type")]
    pub source_type: MediaSourceType,
    pub url: String,
    #[serde(deserialize_with = "required_nullable")]
    pub expires_at: Option<String>,
    pub seekable: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub mime: Option<String>,
}

impl MediaSourceDescriptor {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.url.len() > 8192
            || !(self.url.starts_with("https://") || self.url.starts_with("http://")
                || self.url.starts_with('/'))
        {
            return Err("Media source must be an HTTP URL or a Host route");
        }
        if self.mime.as_ref().is_some_and(|mime| mime.len() > 256) {
            return Err("Media MIME type exceeds bounds");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FetchCoverRequest {
    pub source: SourceRef,
}

/// A Host-scoped resource; its bytes are never embedded in JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverAssetRef {
    pub resource: String,
    pub mime_type: String,
    pub length: u64,
}

impl CoverAssetRef {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.resource.trim().is_empty() || self.resource.len() > 256
            || self.mime_type.trim().is_empty() || self.mime_type.len() > 256
            || self.length == 0 || self.length > 20 * 1024 * 1024
        {
            return Err("Cover must be a bounded Host resource");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_output_requires_declared_null_fields() {
        let item = serde_json::json!({"id": null, "title": "Book"});
        assert!(serde_json::from_value::<ScraperResult>(item).is_err());
        let request = SearchRequest {
            title: Some("Book".into()), author: None, narrator: None,
            page: 1, page_size: 20, filters: Default::default(),
            chapter_candidates: Vec::new(), context: None,
        };
        assert!(request.validate().is_ok());
        assert!(SearchRequest { page_size: 0, ..request }.validate().is_err());
    }

    #[test]
    fn optional_operations_reject_undeclared_fields_and_invalid_resources() {
        let chapter = serde_json::json!({
            "id": "episode-1", "title": "One", "index": 0,
            "duration": null, "source_url": null
        });
        let page = serde_json::json!({
            "items": [chapter], "page": 1, "page_size": 20,
            "total": null, "has_more": null
        });
        assert!(serde_json::from_value::<ChapterPage>(page.clone()).unwrap().validate().is_ok());
        let mut old = page;
        old["items"][0]["book_id"] = serde_json::json!("undeclared");
        assert!(serde_json::from_value::<ChapterPage>(old).is_err());
        assert!(serde_json::from_value::<CoverAssetRef>(serde_json::json!({
            "resource": "host:cover", "mime_type": "image/jpeg", "length": 1
        })).unwrap().validate().is_ok());
        assert!(serde_json::from_value::<CoverAssetRef>(serde_json::json!({
            "resource": "host:cover", "mime_type": "image/jpeg", "length": 0
        })).unwrap().validate().is_err());
        assert!(serde_json::from_value::<MediaSourceDescriptor>(serde_json::json!({
            "type": "direct", "url": "file:///secret", "expires_at": null,
            "mime": null, "seekable": true
        })).unwrap().validate().is_err());
    }
}
