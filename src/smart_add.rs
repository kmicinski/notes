//! Smart Add feature for intelligent paper/reference management.
//!
//! This module handles the Smart Add feature which can:
//! - Detect input types (arXiv URLs, DOIs, generic URLs, plain text)
//! - Query external APIs (arXiv, CrossRef) for metadata
//! - Extract metadata from web pages
//! - Create paper notes with proper frontmatter

use crate::auth::is_logged_in;
use crate::models::{
    AttachSourceRequest, BibImportAnalysis, BibImportConflict, BibImportCreatedNote,
    BibImportEntry, BibImportExecuteRequest, BibImportExecuteResult, BibImportExisting,
    BibImportUpdatedNote, ExternalResult, InputType, LocalMatch, Note, NoteType,
    QuickNoteRequest, SmartAddCreateRequest, SmartAddRequest, SmartAddResult,
};
use crate::notes::{generate_key, normalize_bibtex, normalize_title, parse_bibtex, split_bib_file};
use crate::{validate_path_within, AppState};
use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use crate::url_validator::validate_url;

// ============================================================================
// Input Detection
// ============================================================================

pub fn detect_input_type(input: &str) -> InputType {
    let input = input.trim();

    // Check for arXiv patterns
    if let Some(arxiv_id) = extract_arxiv_id(input) {
        return InputType::ArxivUrl { arxiv_id };
    }

    // Check for DOI patterns
    if let Some(doi) = extract_doi(input) {
        return InputType::DoiUrl { doi };
    }

    // Check for URL patterns
    if input.starts_with("http://") || input.starts_with("https://") {
        return InputType::GenericUrl {
            url: input.to_string(),
        };
    }

    InputType::PlainText {
        text: input.to_string(),
    }
}

pub fn extract_arxiv_id(input: &str) -> Option<String> {
    // Match arxiv URLs or bare IDs
    // Formats: arxiv.org/abs/2301.00001, arxiv.org/pdf/2301.00001.pdf, 2301.00001, arXiv:2301.00001
    let patterns = [
        r"arxiv\.org/(?:abs|pdf)/(\d{4}\.\d{4,5})",
        r"arxiv\.org/(?:abs|pdf)/([a-z-]+/\d{7})",
        r"arXiv:(\d{4}\.\d{4,5})",
        r"^(\d{4}\.\d{4,5})$",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(input) {
                if let Some(m) = caps.get(1) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

pub fn extract_doi(input: &str) -> Option<String> {
    // Match DOI patterns from various sources
    // DOI format: 10.XXXX/... where XXXX is 4+ digits
    let patterns = [
        // Standard DOI URLs
        r#"(?:doi\.org|dx\.doi\.org)/?(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // ACM Digital Library: dl.acm.org/doi/10.1145/...
        r#"dl\.acm\.org/doi/(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // IEEE Xplore: ieeexplore.ieee.org/document/... doesn't have DOI in URL, skip
        // Springer: link.springer.com/article/10.1007/...
        r#"link\.springer\.com/(?:article|chapter)/(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // Wiley: onlinelibrary.wiley.com/doi/10.1002/...
        r#"onlinelibrary\.wiley\.com/doi/(?:abs/|full/)?(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // Nature: nature.com/articles/... (DOI embedded)
        r#"nature\.com/articles/(10\.\d{4,}/[^\s\]"'<>]+)"#,
        // Science Direct / Elsevier
        r#"sciencedirect\.com/science/article/pii/[^?]+\?.*doi=(10\.\d{4,}/[^\s\]"'<>&]+)"#,
        // PLOS
        r#"journals\.plos\.org/\w+/article\?id=(10\.\d{4,}/[^\s\]"'<>&]+)"#,
        // Generic: any URL containing a DOI pattern
        r#"/(10\.\d{4,}/[^\s\]"'<>/?#]+)"#,
        // Bare DOI
        r#"^(10\.\d{4,}/[^\s\]"'<>]+)$"#,
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(input) {
                if let Some(m) = caps.get(1) {
                    // Clean up the DOI (remove trailing punctuation)
                    let doi = m
                        .as_str()
                        .trim_end_matches(|c| c == '.' || c == ',' || c == ';');
                    return Some(doi.to_string());
                }
            }
        }
    }
    None
}

// ============================================================================
// Local Search
// ============================================================================

pub fn search_local_for_match(
    notes: &[Note],
    input: &str,
    input_type: &InputType,
) -> Option<LocalMatch> {
    let input_lower = input.to_lowercase();

    // Helper to check if a note has a matching source
    let check_source = |note: &Note, source_type: &str, identifier: &str| -> bool {
        if let NoteType::Paper(ref paper) = note.note_type {
            for source in &paper.sources {
                if source.source_type == source_type && source.identifier == identifier {
                    return true;
                }
            }
        }
        // Also check content for backwards compatibility
        note.full_file_content.contains(identifier)
    };

    // For arXiv IDs, check sources and content
    if let InputType::ArxivUrl { arxiv_id } = input_type {
        for note in notes {
            if check_source(note, "arxiv", arxiv_id) {
                return Some(LocalMatch {
                    key: note.key.clone(),
                    title: note.title.clone(),
                    match_type: "arxiv".to_string(),
                });
            }
        }
    }

    // For DOIs, check sources and content
    if let InputType::DoiUrl { doi } = input_type {
        for note in notes {
            if check_source(note, "doi", doi) {
                return Some(LocalMatch {
                    key: note.key.clone(),
                    title: note.title.clone(),
                    match_type: "doi".to_string(),
                });
            }
        }
    }

    // For any URL, try to extract a DOI and search for it
    if let Some(doi) = extract_doi(input) {
        for note in notes {
            if check_source(note, "doi", &doi) {
                return Some(LocalMatch {
                    key: note.key.clone(),
                    title: note.title.clone(),
                    match_type: "doi".to_string(),
                });
            }
        }
    }

    // Check for title matches
    for note in notes {
        let note_title_lower = note.title.to_lowercase();

        // Exact title match
        if note_title_lower == input_lower {
            return Some(LocalMatch {
                key: note.key.clone(),
                title: note.title.clone(),
                match_type: "exact".to_string(),
            });
        }
    }

    // Fuzzy title match (for papers, check title match)
    for note in notes {
        if let NoteType::Paper(_) = note.note_type {
            let note_title_lower = note.title.to_lowercase();

            // Check if input contains significant portion of title
            let title_words: Vec<&str> = note_title_lower.split_whitespace().collect();
            let input_words: Vec<&str> = input_lower.split_whitespace().collect();

            if title_words.len() >= 3 {
                let matching_words = title_words
                    .iter()
                    .filter(|w| w.len() > 3 && input_words.contains(w))
                    .count();

                if matching_words >= title_words.len() * 2 / 3 {
                    // Good title match found
                    return Some(LocalMatch {
                        key: note.key.clone(),
                        title: note.title.clone(),
                        match_type: "title".to_string(),
                    });
                }
            }
        }
    }

    None
}

// ============================================================================
// Metadata Generation
// ============================================================================

pub fn generate_bib_key(title: &str, authors: Option<&str>, year: Option<i32>) -> String {
    // Format: lastname + year + keyword
    // Example: vaswani2017attention

    let lastname = authors
        .and_then(|a| a.split(',').next())
        .and_then(|a| a.split_whitespace().last())
        .unwrap_or("unknown")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>();

    let year_str = year.map(|y| y.to_string()).unwrap_or_default();

    // Get first significant word from title (skip common words)
    let skip_words = [
        "a", "an", "the", "on", "of", "for", "to", "in", "with", "and", "is", "are",
    ];
    let keyword = title
        .split_whitespace()
        .find(|w| !skip_words.contains(&w.to_lowercase().as_str()) && w.len() > 2)
        .unwrap_or("paper")
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect::<String>();

    format!("{}{}{}", lastname, year_str, keyword)
}

pub fn generate_suggested_filename(title: &str) -> String {
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-");

    format!("{}.md", slug)
}

// ============================================================================
// External API Integration
// ============================================================================

pub async fn query_arxiv_api(arxiv_id: &str) -> Option<ExternalResult> {
    let url = format!("https://export.arxiv.org/api/query?id_list={}", arxiv_id);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(_) => return None,
    };

    let text = match response.text().await {
        Ok(t) => t,
        Err(_) => return None,
    };

    // Parse XML response - need to extract from <entry> not the feed
    // The feed has its own <title> which is "arXiv Query: ..."
    // We want the <title> inside <entry>
    let entry = extract_xml_tag(&text, "entry")?;

    let title = extract_xml_tag(&entry, "title")
        .map(|t| t.trim().replace('\n', " ").replace("  ", " "))
        .filter(|t| !t.starts_with("Error") && !t.is_empty())?;

    let authors: Vec<String> = extract_all_xml_tags(&entry, "name")
        .into_iter()
        .map(|s| s.trim().to_string())
        .collect();
    let authors_str = if authors.is_empty() {
        None
    } else {
        Some(authors.join(" and "))
    };

    let published = extract_xml_tag(&entry, "published");
    let year = published.and_then(|p| p.get(..4).and_then(|y| y.parse().ok()));

    let bib_key = generate_bib_key(&title, authors_str.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    // Generate bibtex
    let bibtex = format!(
        "@article{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  eprint = {{{}}},\n  archivePrefix = {{arXiv}},\n}}",
        bib_key,
        title,
        authors_str.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        arxiv_id
    );

    Some(ExternalResult {
        title,
        authors: authors_str,
        year,
        venue: Some("arXiv".to_string()),
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "arxiv".to_string(),
    })
}

pub async fn query_crossref_api(doi: &str) -> Option<ExternalResult> {
    let url = format!("https://api.crossref.org/works/{}", doi);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client
        .get(&url)
        .header("User-Agent", "NotesApp/1.0 (mailto:user@example.com)")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(_) => return None,
    };

    let message = json.get("message")?;

    let title = message
        .get("title")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())?;

    let authors: Vec<String> = message
        .get("author")
        .and_then(|a| a.as_array())
        .map(|authors| {
            authors
                .iter()
                .filter_map(|a| {
                    let given = a.get("given").and_then(|g| g.as_str()).unwrap_or("");
                    let family = a.get("family").and_then(|f| f.as_str()).unwrap_or("");
                    if family.is_empty() {
                        None
                    } else {
                        Some(format!("{} {}", given, family).trim().to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let authors_str = if authors.is_empty() {
        None
    } else {
        Some(authors.join(" and "))
    };

    let year = message
        .get("published")
        .or_else(|| message.get("published-print"))
        .or_else(|| message.get("published-online"))
        .and_then(|p| p.get("date-parts"))
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .and_then(|y| y.as_i64())
        .map(|y| y as i32);

    let venue = message
        .get("container-title")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let bib_key = generate_bib_key(&title, authors_str.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    // Generate bibtex
    let bibtex = format!(
        "@article{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  doi = {{{}}},\n  journal = {{{}}},\n}}",
        bib_key,
        title,
        authors_str.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        doi,
        venue.as_deref().unwrap_or("")
    );

    Some(ExternalResult {
        title,
        authors: authors_str,
        year,
        venue,
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "crossref".to_string(),
    })
}

pub async fn query_crossref_by_title(title: &str) -> Option<ExternalResult> {
    let encoded_title = urlencoding::encode(title);
    let url = format!(
        "https://api.crossref.org/works?query.title={}&rows=1",
        encoded_title
    );

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let response = match client
        .get(&url)
        .header("User-Agent", "NotesApp/1.0 (mailto:user@example.com)")
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return None,
    };

    let json: serde_json::Value = match response.json().await {
        Ok(j) => j,
        Err(_) => return None,
    };

    let items = json.get("message")?.get("items")?.as_array()?;
    let item = items.first()?;

    let found_title = item
        .get("title")
        .and_then(|t| t.as_array())
        .and_then(|a| a.first())
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())?;

    // Check if title is similar enough
    let title_lower = title.to_lowercase();
    let found_lower = found_title.to_lowercase();
    if !found_lower.contains(&title_lower) && !title_lower.contains(&found_lower) {
        // Titles too different
        return None;
    }

    let doi = item.get("DOI").and_then(|d| d.as_str())?;
    query_crossref_api(doi).await
}

/// Fetch a URL and extract paper metadata from HTML meta tags
pub async fn fetch_and_extract_metadata(url: &str) -> Option<ExternalResult> {
    // Validate URL for SSRF protection
    if validate_url(url).is_err() {
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; NotesApp/1.0)")
        .build()
        .ok()?;

    let response = client.get(url).send().await.ok()?;
    let html = response.text().await.ok()?;

    // First, try to extract DOI from the page and use CrossRef
    if let Some(doi) = extract_doi_from_html(&html) {
        if let Some(result) = query_crossref_api(&doi).await {
            return Some(result);
        }
    }

    // Extract metadata from HTML meta tags
    let title = extract_meta_content(
        &html,
        &[
            "citation_title",
            "DC.title",
            "og:title",
            "twitter:title",
        ],
    )
    .or_else(|| extract_html_title(&html))?;

    // Skip if title looks like an error page
    if title.to_lowercase().contains("access denied")
        || title.to_lowercase().contains("404")
        || title.to_lowercase().contains("not found")
    {
        return None;
    }

    let authors = extract_meta_content(&html, &["citation_author", "DC.creator", "author"])
        .or_else(|| extract_all_meta_content(&html, "citation_author"));

    let year = extract_meta_content(
        &html,
        &[
            "citation_publication_date",
            "citation_date",
            "DC.date",
            "article:published_time",
        ],
    )
    .and_then(|d| d.get(..4).and_then(|y| y.parse().ok()));

    let venue = extract_meta_content(
        &html,
        &[
            "citation_journal_title",
            "citation_conference_title",
            "DC.publisher",
            "og:site_name",
        ],
    );

    let bib_key = generate_bib_key(&title, authors.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    let bibtex = format!(
        "@misc{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  howpublished = {{\\url{{{}}}}},\n}}",
        bib_key,
        title,
        authors.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        url
    );

    Some(ExternalResult {
        title,
        authors,
        year,
        venue,
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "webpage".to_string(),
    })
}

fn extract_doi_from_html(html: &str) -> Option<String> {
    // Look for DOI in meta tags
    let doi_patterns = [
        r#"name="citation_doi"\s+content="([^"]+)""#,
        r#"name="DC.identifier"\s+content="([^"]+)""#,
        r#"name="doi"\s+content="([^"]+)""#,
        r#"content="([^"]+)"\s+name="citation_doi""#,
        r#"data-doi="([^"]+)""#,
    ];

    for pattern in doi_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(html) {
                if let Some(m) = caps.get(1) {
                    let doi = m.as_str();
                    // Validate it looks like a DOI
                    if doi.starts_with("10.") {
                        return Some(doi.to_string());
                    }
                }
            }
        }
    }

    // Also look for DOI in text content
    if let Ok(re) =
        Regex::new(r#"(?:doi|DOI)[:\s]+(?:https?://(?:dx\.)?doi\.org/)?(10\.\d{4,}/[^\s<"']+)"#)
    {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                return Some(
                    m.as_str()
                        .trim_end_matches(|c| c == '.' || c == ',')
                        .to_string(),
                );
            }
        }
    }

    None
}

fn extract_meta_content(html: &str, names: &[&str]) -> Option<String> {
    for name in names {
        // Try both name="X" content="Y" and content="Y" name="X" orders
        let patterns = [
            format!(
                r#"(?i)<meta[^>]*name=["']{}["'][^>]*content=["']([^"']+)["']"#,
                regex::escape(name)
            ),
            format!(
                r#"(?i)<meta[^>]*content=["']([^"']+)["'][^>]*name=["']{}["']"#,
                regex::escape(name)
            ),
            format!(
                r#"(?i)<meta[^>]*property=["']{}["'][^>]*content=["']([^"']+)["']"#,
                regex::escape(name)
            ),
            format!(
                r#"(?i)<meta[^>]*content=["']([^"']+)["'][^>]*property=["']{}["']"#,
                regex::escape(name)
            ),
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    if let Some(m) = caps.get(1) {
                        let content = m.as_str().trim();
                        if !content.is_empty() {
                            return Some(html_entity_decode(content));
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_all_meta_content(html: &str, name: &str) -> Option<String> {
    let pattern = format!(
        r#"(?i)<meta[^>]*name=["']{}["'][^>]*content=["']([^"']+)["']"#,
        regex::escape(name)
    );
    let mut authors = Vec::new();

    if let Ok(re) = Regex::new(&pattern) {
        for caps in re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                authors.push(html_entity_decode(m.as_str().trim()));
            }
        }
    }

    if authors.is_empty() {
        None
    } else {
        Some(authors.join(" and "))
    }
}

fn extract_html_title(html: &str) -> Option<String> {
    let pattern = r"(?i)<title[^>]*>([^<]+)</title>";
    if let Ok(re) = Regex::new(pattern) {
        if let Some(caps) = re.captures(html) {
            if let Some(m) = caps.get(1) {
                let title = html_entity_decode(m.as_str().trim());
                // Clean up common suffixes
                let title = title
                    .split(" | ")
                    .next()
                    .unwrap_or(&title)
                    .split(" - ")
                    .next()
                    .unwrap_or(&title)
                    .trim();
                if !title.is_empty() {
                    return Some(title.to_string());
                }
            }
        }
    }
    None
}

fn html_entity_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
}

pub async fn query_claude_for_url(url: &str) -> Option<ExternalResult> {
    let prompt = format!(
        "Extract paper/article metadata from this URL: {}\n\n\
        Return ONLY a JSON object with these fields (no other text):\n\
        {{\"title\": \"...\", \"authors\": \"Author1 and Author2\", \"year\": 2024, \"venue\": \"...\"}}\n\n\
        If you cannot access or parse the URL, return: {{\"error\": \"cannot access\"}}",
        url
    );

    let output = tokio::task::spawn_blocking(move || {
        Command::new("claude").args(["-p", &prompt]).output()
    })
    .await
    .ok()?
    .ok()?;

    if !output.status.success() {
        return None;
    }

    let response = String::from_utf8_lossy(&output.stdout);

    // Try to extract JSON from response
    let json_start = response.find('{')?;
    let json_end = response.rfind('}')?;
    let json_str = &response[json_start..=json_end];

    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;

    if json.get("error").is_some() {
        return None;
    }

    let title = json
        .get("title")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())?;
    let authors = json
        .get("authors")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string());
    let year = json.get("year").and_then(|y| y.as_i64()).map(|y| y as i32);
    let venue = json
        .get("venue")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let bib_key = generate_bib_key(&title, authors.as_deref(), year);
    let suggested_filename = generate_suggested_filename(&title);

    let bibtex = format!(
        "@misc{{{},\n  title = {{{}}},\n  author = {{{}}},\n  year = {{{}}},\n  howpublished = {{\\url{{{}}}}},\n}}",
        bib_key,
        title,
        authors.as_deref().unwrap_or(""),
        year.unwrap_or(0),
        url
    );

    Some(ExternalResult {
        title,
        authors,
        year,
        venue,
        bib_key,
        bibtex: Some(bibtex),
        suggested_filename,
        source: "claude".to_string(),
    })
}

// ============================================================================
// XML Helpers
// ============================================================================

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);

    let start = xml.find(&start_tag)? + start_tag.len();
    let end = xml[start..].find(&end_tag)? + start;

    Some(xml[start..end].to_string())
}

fn extract_all_xml_tags(xml: &str, tag: &str) -> Vec<String> {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    let mut results = Vec::new();
    let mut search_start = 0;

    while let Some(start_pos) = xml[search_start..].find(&start_tag) {
        let abs_start = search_start + start_pos + start_tag.len();
        if let Some(end_pos) = xml[abs_start..].find(&end_tag) {
            results.push(xml[abs_start..abs_start + end_pos].to_string());
            search_start = abs_start + end_pos + end_tag.len();
        } else {
            break;
        }
    }

    results
}

// ============================================================================
// Route Handlers
// ============================================================================

pub async fn smart_add_lookup(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<SmartAddRequest>,
) -> Response {
    // Always return JSON responses for consistency
    if !is_logged_in(&jar, &state.db) {
        let result = SmartAddResult {
            input_type: "error".to_string(),
            local_match: None,
            external_result: None,
            error: Some("Not logged in".to_string()),
        };
        return axum::Json(result).into_response();
    }

    let input = body.input.trim().to_string();
    if input.is_empty() {
        let result = SmartAddResult {
            input_type: "empty".to_string(),
            local_match: None,
            external_result: None,
            error: Some("Input is empty".to_string()),
        };
        return axum::Json(result).into_response();
    }

    let input_type = detect_input_type(&input);
    let notes = state.load_notes();

    // Check for local match first
    let local_match = search_local_for_match(&notes, &input, &input_type);

    // Query external APIs based on input type (with error handling)
    let external_result = match &input_type {
        InputType::ArxivUrl { arxiv_id } => {
            let arxiv_id = arxiv_id.clone();
            // Try arXiv API, fallback to Claude, then None
            match query_arxiv_api(&arxiv_id).await {
                Some(r) => Some(r),
                None => {
                    // Claude fallback is optional - don't fail if it's not available
                    query_claude_for_url(&format!("https://arxiv.org/abs/{}", arxiv_id)).await
                }
            }
        }
        InputType::DoiUrl { doi } => {
            let doi = doi.clone();
            // Try CrossRef API, fallback to Claude
            match query_crossref_api(&doi).await {
                Some(r) => Some(r),
                None => query_claude_for_url(&format!("https://doi.org/{}", doi)).await,
            }
        }
        InputType::GenericUrl { url } => {
            // Try to fetch and extract metadata from the page
            match fetch_and_extract_metadata(url).await {
                Some(r) => Some(r),
                None => query_claude_for_url(url).await,
            }
        }
        InputType::PlainText { text } => {
            // Try CrossRef title search
            query_crossref_by_title(text).await
        }
    };

    let input_type_str = match &input_type {
        InputType::ArxivUrl { .. } => "arxiv",
        InputType::DoiUrl { .. } => "doi",
        InputType::GenericUrl { .. } => "url",
        InputType::PlainText { .. } => "text",
    };

    let result = SmartAddResult {
        input_type: input_type_str.to_string(),
        local_match,
        external_result,
        error: None,
    };

    axum::Json(result).into_response()
}

#[derive(Serialize)]
pub struct SmartAddCreateResponse {
    pub key: Option<String>,
    pub error: Option<String>,
}

pub async fn smart_add_create(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<SmartAddCreateRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Not logged in".to_string()),
        })
        .into_response();
    }

    // BibTeX is required and must be parseable
    let bibtex = body.bibtex.trim().to_string();
    if bibtex.is_empty() {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("BibTeX is required".to_string()),
        })
        .into_response();
    }

    let parsed = match parse_bibtex(&bibtex) {
        Some(p) => p,
        None => {
            return axum::Json(SmartAddCreateResponse {
                key: None,
                error: Some("Could not parse BibTeX entry".to_string()),
            })
            .into_response();
        }
    };

    let title = parsed.title.unwrap_or_else(|| parsed.cite_key.clone());
    let filename = body.filename.trim();

    // Validate filename
    if filename.is_empty() || !filename.ends_with(".md") {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Filename must end with .md".to_string()),
        })
        .into_response();
    }

    // Check for path traversal: reject .., absolute paths, and null bytes
    if filename.contains("..") || filename.starts_with('/') || filename.contains('\0') {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Invalid filename".to_string()),
        })
        .into_response();
    }

    let file_path = state.notes_dir.join(filename);

    // Validate the path stays within notes_dir
    if let Err(_) = validate_path_within(&state.notes_dir, &file_path) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Invalid filename".to_string()),
        })
        .into_response();
    }

    // Check if file exists
    if file_path.exists() {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some(format!(
                "A note with filename '{}' already exists",
                filename
            )),
        })
        .into_response();
    }

    // Create parent directories if needed
    if let Some(parent) = file_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return axum::Json(SmartAddCreateResponse {
                key: None,
                error: Some(format!("Failed to create directory: {}", e)),
            })
            .into_response();
        }
    }

    // Build frontmatter — title and bibtex are the key fields;
    // all other metadata (authors, year, venue) is derived from bibtex at read time
    let today = Utc::now().format("%Y-%m-%d");
    let mut frontmatter = format!(
        "---\ntitle: {}\ndate: {}\ntype: paper\nbibtex: |\n",
        title, today
    );
    for line in bibtex.lines() {
        frontmatter.push_str(&format!("  {}\n", line));
    }
    if let Some(ref arxiv_id) = body.arxiv_id {
        if !arxiv_id.is_empty() {
            frontmatter.push_str(&format!("arxiv: {}\n", arxiv_id));
        }
    }
    if let Some(ref doi) = body.doi {
        if !doi.is_empty() {
            frontmatter.push_str(&format!("doi: {}\n", doi));
        }
    }

    frontmatter.push_str("---\n\n## Summary\n\n## Key Contributions\n\n## Notes\n\n");

    // Write the file
    if let Err(e) = fs::write(&file_path, &frontmatter) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some(format!("Failed to create note: {}", e)),
        })
        .into_response();
    }

    // Generate key for the new note
    let relative_path = PathBuf::from(filename);
    let key = generate_key(&relative_path);

    axum::Json(SmartAddCreateResponse {
        key: Some(key),
        error: None,
    })
    .into_response()
}

pub async fn quick_note_create(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<QuickNoteRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Not logged in".to_string()),
        })
        .into_response();
    }

    let title = body.title.trim().to_string();
    if title.is_empty() {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Title is required".to_string()),
        })
        .into_response();
    }

    // Generate slug from title
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join("-");

    // Build filename with optional subdirectory
    let filename = if let Some(ref subdir) = body.subdirectory {
        let subdir = subdir.trim().trim_matches('/');
        if subdir.is_empty() {
            format!("{}.md", slug)
        } else {
            format!("{}/{}.md", subdir, slug)
        }
    } else {
        format!("{}.md", slug)
    };

    // Validate filename
    if filename.contains("..") || filename.starts_with('/') || filename.contains('\0') {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Invalid filename".to_string()),
        })
        .into_response();
    }

    let file_path = state.notes_dir.join(&filename);

    if let Err(_) = validate_path_within(&state.notes_dir, &file_path) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Invalid filename".to_string()),
        })
        .into_response();
    }

    if file_path.exists() {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some(format!("A note with filename '{}' already exists", filename)),
        })
        .into_response();
    }

    if let Some(parent) = file_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return axum::Json(SmartAddCreateResponse {
                key: None,
                error: Some(format!("Failed to create directory: {}", e)),
            })
            .into_response();
        }
    }

    let date = body
        .date
        .as_deref()
        .filter(|d| !d.is_empty())
        .unwrap_or(&Utc::now().format("%Y-%m-%d").to_string())
        .to_string();

    let frontmatter = format!("---\ntitle: {}\ndate: {}\n---\n\n", title, date);

    if let Err(e) = fs::write(&file_path, &frontmatter) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some(format!("Failed to create note: {}", e)),
        })
        .into_response();
    }

    let relative_path = PathBuf::from(&filename);
    let key = generate_key(&relative_path);

    axum::Json(SmartAddCreateResponse {
        key: Some(key),
        error: None,
    })
    .into_response()
}

#[derive(Serialize)]
pub struct AttachSourceResponse {
    pub success: bool,
    pub error: Option<String>,
}

pub async fn smart_add_attach(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<AttachSourceRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return axum::Json(AttachSourceResponse {
            success: false,
            error: Some("Not logged in".to_string()),
        })
        .into_response();
    }

    let notes_map = state.notes_map();
    let note = match notes_map.get(&body.note_key) {
        Some(n) => n,
        None => {
            return axum::Json(AttachSourceResponse {
                success: false,
                error: Some("Note not found".to_string()),
            })
            .into_response()
        }
    };

    // Read the current file content
    let full_path = state.notes_dir.join(&note.path);
    let content = match fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => {
            return axum::Json(AttachSourceResponse {
                success: false,
                error: Some(format!("Failed to read note: {}", e)),
            })
            .into_response()
        }
    };

    // Add the new source to frontmatter
    let source_line = match body.source_type.as_str() {
        "arxiv" => format!("arxiv: {}", body.identifier),
        "doi" => format!("doi: {}", body.identifier),
        _ => format!("url: {}", body.identifier),
    };

    // Find the end of frontmatter and insert before ---
    let new_content = if let Some(second_dash) = content
        .find("---")
        .and_then(|first| content[first + 3..].find("---").map(|second| first + 3 + second))
    {
        // Insert the source line before the closing ---
        let mut new = content[..second_dash].to_string();
        // Make sure there's a newline
        if !new.ends_with('\n') {
            new.push('\n');
        }
        new.push_str(&source_line);
        new.push('\n');
        new.push_str(&content[second_dash..]);
        new
    } else {
        return axum::Json(AttachSourceResponse {
            success: false,
            error: Some("Could not find frontmatter".to_string()),
        })
        .into_response();
    };

    // Write the updated content
    if let Err(e) = fs::write(&full_path, &new_content) {
        return axum::Json(AttachSourceResponse {
            success: false,
            error: Some(format!("Failed to write note: {}", e)),
        })
        .into_response();
    }

    axum::Json(AttachSourceResponse {
        success: true,
        error: None,
    })
    .into_response()
}

// ============================================================================
// BibTeX Import Endpoints
// ============================================================================

/// Insert a text block before the closing `---` of frontmatter.
fn insert_before_frontmatter_end(content: &str, block: &str) -> Option<String> {
    let first = content.find("---")?;
    let second = content[first + 3..].find("---").map(|i| first + 3 + i)?;
    let mut new = content[..second].to_string();
    if !new.ends_with('\n') {
        new.push('\n');
    }
    new.push_str(block);
    if !block.ends_with('\n') {
        new.push('\n');
    }
    new.push_str(&content[second..]);
    Some(new)
}

pub async fn bib_import_analyze(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    // Read the .bib file from multipart
    let mut file_content = String::new();
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            match field.text().await {
                Ok(text) => {
                    file_content = text;
                    break;
                }
                Err(e) => {
                    return axum::Json(BibImportAnalysis {
                        new_entries: vec![],
                        existing_entries: vec![],
                        conflicts: vec![],
                        parse_errors: vec![format!("Failed to read file: {}", e)],
                    })
                    .into_response();
                }
            }
        }
    }

    if file_content.is_empty() {
        return axum::Json(BibImportAnalysis {
            new_entries: vec![],
            existing_entries: vec![],
            conflicts: vec![],
            parse_errors: vec!["No file uploaded".to_string()],
        })
        .into_response();
    }

    let raw_entries = split_bib_file(&file_content);
    let notes = state.load_notes();

    // Build lookup indexes from existing notes
    let mut cite_key_to_note: HashMap<String, (String, String, Option<String>)> = HashMap::new(); // cite_key -> (note_key, note_title, bibtex)
    let mut doi_to_note: HashMap<String, (String, String)> = HashMap::new();
    let mut title_to_note: HashMap<String, (String, String)> = HashMap::new(); // normalized_title -> (note_key, note_title)

    for note in &notes {
        if let NoteType::Paper(ref paper) = note.note_type {
            for entry in &paper.bibtex_entries {
                if let Some(parsed) = parse_bibtex(entry) {
                    cite_key_to_note.insert(
                        parsed.cite_key.clone(),
                        (note.key.clone(), note.title.clone(), Some(entry.clone())),
                    );
                    if let Some(ref doi) = parsed.doi {
                        doi_to_note.insert(doi.to_lowercase(), (note.key.clone(), note.title.clone()));
                    }
                    if let Some(ref title) = parsed.title {
                        let norm = normalize_title(title);
                        if !norm.is_empty() {
                            title_to_note.insert(norm, (note.key.clone(), note.title.clone()));
                        }
                    }
                }
            }
            // Also index the note title itself
            let norm = normalize_title(&note.title);
            if !norm.is_empty() {
                title_to_note.entry(norm).or_insert_with(|| (note.key.clone(), note.title.clone()));
            }
        }
    }

    let mut analysis = BibImportAnalysis {
        new_entries: vec![],
        existing_entries: vec![],
        conflicts: vec![],
        parse_errors: vec![],
    };

    for (idx, raw) in raw_entries.into_iter().enumerate() {
        let parsed = match parse_bibtex(&raw) {
            Some(p) => p,
            None => {
                analysis.parse_errors.push(format!(
                    "Entry {}: could not parse BibTeX",
                    idx + 1
                ));
                continue;
            }
        };

        let cite_key = &parsed.cite_key;

        // Check cite key match
        if let Some((note_key, note_title, existing_bib)) = cite_key_to_note.get(cite_key) {
            // Same cite key exists - check if content is identical
            if let Some(ref existing) = existing_bib {
                if normalize_bibtex(existing) == normalize_bibtex(&raw) {
                    // Identical — skip
                    analysis.existing_entries.push(BibImportExisting {
                        index: idx,
                        cite_key: cite_key.clone(),
                        note_key: note_key.clone(),
                        note_title: note_title.clone(),
                    });
                    continue;
                }
            }
            // Cite key matches but content differs — conflict
            analysis.conflicts.push(BibImportConflict {
                index: idx,
                bibtex: raw,
                cite_key: cite_key.clone(),
                title: parsed.title.clone(),
                match_type: "cite_key".to_string(),
                matched_note_key: note_key.clone(),
                matched_note_title: note_title.clone(),
                existing_bibtex: existing_bib.clone(),
            });
            continue;
        }

        // Check DOI match
        if let Some(ref doi) = parsed.doi {
            if let Some((note_key, note_title)) = doi_to_note.get(&doi.to_lowercase()) {
                analysis.conflicts.push(BibImportConflict {
                    index: idx,
                    bibtex: raw,
                    cite_key: cite_key.clone(),
                    title: parsed.title.clone(),
                    match_type: "doi".to_string(),
                    matched_note_key: note_key.clone(),
                    matched_note_title: note_title.clone(),
                    existing_bibtex: None,
                });
                continue;
            }
        }

        // Check title match
        if let Some(ref title) = parsed.title {
            let norm = normalize_title(title);
            if !norm.is_empty() {
                if let Some((note_key, note_title)) = title_to_note.get(&norm) {
                    analysis.conflicts.push(BibImportConflict {
                        index: idx,
                        bibtex: raw,
                        cite_key: cite_key.clone(),
                        title: parsed.title.clone(),
                        match_type: "title".to_string(),
                        matched_note_key: note_key.clone(),
                        matched_note_title: note_title.clone(),
                        existing_bibtex: None,
                    });
                    continue;
                }
            }
        }

        // New entry
        analysis.new_entries.push(BibImportEntry {
            index: idx,
            bibtex: raw,
            cite_key: cite_key.clone(),
            title: parsed.title.clone(),
            author: parsed.author.clone(),
            year: parsed.year,
            suggested_filename: format!("{}.md", cite_key),
        });
    }

    axum::Json(analysis).into_response()
}

pub async fn bib_import_execute(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(body): axum::Json<BibImportExecuteRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let mut result = BibImportExecuteResult {
        created: vec![],
        updated: vec![],
        errors: vec![],
    };

    // Process create items
    for item in &body.create {
        let bibtex = item.bibtex.trim();
        let filename = item.filename.trim();

        if filename.is_empty() || !filename.ends_with(".md") {
            result.errors.push(format!("Invalid filename: {}", filename));
            continue;
        }

        if filename.contains("..") || filename.starts_with('/') || filename.contains('\0') {
            result.errors.push(format!("Invalid filename: {}", filename));
            continue;
        }

        let parsed = match parse_bibtex(bibtex) {
            Some(p) => p,
            None => {
                result.errors.push(format!("Could not parse BibTeX for {}", filename));
                continue;
            }
        };

        let title = parsed.title.unwrap_or_else(|| parsed.cite_key.clone());
        let file_path = state.notes_dir.join(filename);

        if let Err(_) = validate_path_within(&state.notes_dir, &file_path) {
            result.errors.push(format!("Invalid filename: {}", filename));
            continue;
        }

        if file_path.exists() {
            result.errors.push(format!("File already exists: {}", filename));
            continue;
        }

        if let Some(parent) = file_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                result.errors.push(format!("Failed to create directory for {}: {}", filename, e));
                continue;
            }
        }

        let today = chrono::Utc::now().format("%Y-%m-%d");
        let mut frontmatter = format!(
            "---\ntitle: {}\ndate: {}\ntype: paper\nbibtex: |\n",
            title, today
        );
        for line in bibtex.lines() {
            frontmatter.push_str(&format!("  {}\n", line));
        }

        // Extract DOI from bibtex if present
        if let Some(ref doi) = parsed.doi {
            if !doi.is_empty() {
                frontmatter.push_str(&format!("doi: {}\n", doi));
            }
        }

        frontmatter.push_str("---\n\n## Summary\n\n## Key Contributions\n\n## Notes\n\n");

        if let Err(e) = fs::write(&file_path, &frontmatter) {
            result.errors.push(format!("Failed to write {}: {}", filename, e));
            continue;
        }

        let relative_path = PathBuf::from(filename);
        let key = generate_key(&relative_path);

        result.created.push(BibImportCreatedNote {
            key,
            filename: filename.to_string(),
            title,
        });
    }

    // Process secondary items (add bibtex to existing notes)
    let notes_map = state.notes_map();
    for item in &body.add_secondary {
        let note = match notes_map.get(&item.note_key) {
            Some(n) => n,
            None => {
                result.errors.push(format!("Note not found: {}", item.note_key));
                continue;
            }
        };

        let full_path = state.notes_dir.join(&note.path);
        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(e) => {
                result.errors.push(format!("Failed to read {}: {}", note.title, e));
                continue;
            }
        };

        // Build the bibtex block to insert
        let mut block = String::from("bibtex: |\n");
        for line in item.bibtex.trim().lines() {
            block.push_str(&format!("  {}\n", line));
        }

        let new_content = match insert_before_frontmatter_end(&content, &block) {
            Some(c) => c,
            None => {
                result.errors.push(format!("Could not find frontmatter in {}", note.title));
                continue;
            }
        };

        if let Err(e) = fs::write(&full_path, &new_content) {
            result.errors.push(format!("Failed to update {}: {}", note.title, e));
            continue;
        }

        result.updated.push(BibImportUpdatedNote {
            key: note.key.clone(),
            title: note.title.clone(),
        });
    }

    axum::Json(result).into_response()
}
