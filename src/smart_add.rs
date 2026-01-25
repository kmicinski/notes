//! Smart Add feature for intelligent paper/reference management.
//!
//! This module handles the Smart Add feature which can:
//! - Detect input types (arXiv URLs, DOIs, generic URLs, plain text)
//! - Query external APIs (arXiv, CrossRef) for metadata
//! - Extract metadata from web pages
//! - Create paper notes with proper frontmatter

use crate::auth::is_logged_in;
use crate::models::{
    AttachSourceRequest, ExternalResult, InputType, LocalMatch, Note, NoteType,
    SmartAddCreateRequest, SmartAddRequest, SmartAddResult,
};
use crate::notes::generate_key;
use crate::AppState;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

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

    // Fuzzy title match (for papers, check title + authors + year)
    for note in notes {
        if let NoteType::Paper(ref paper) = note.note_type {
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
                    // Also check author/year if available
                    let mut score = matching_words;
                    if let Some(ref authors) = paper.authors {
                        if input_lower.contains(
                            &authors
                                .to_lowercase()
                                .split(',')
                                .next()
                                .unwrap_or("")
                                .trim()
                                .to_lowercase(),
                        ) {
                            score += 2;
                        }
                    }
                    if let Some(year) = paper.year {
                        if input.contains(&year.to_string()) {
                            score += 2;
                        }
                    }
                    if score >= title_words.len() * 2 / 3 + 2 {
                        return Some(LocalMatch {
                            key: note.key.clone(),
                            title: note.title.clone(),
                            match_type: "title".to_string(),
                        });
                    }
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
    if !is_logged_in(&jar) {
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
    if !is_logged_in(&jar) {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Not logged in".to_string()),
        })
        .into_response();
    }

    // Validate required fields
    if body.title.trim().is_empty()
        || body.filename.trim().is_empty()
        || body.bib_key.trim().is_empty()
    {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Title, filename, and bib_key are required".to_string()),
        })
        .into_response();
    }

    let filename = body.filename.trim();

    // Validate filename
    if !filename.ends_with(".md") {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Filename must end with .md".to_string()),
        })
        .into_response();
    }

    // Check for path traversal
    if filename.contains("..") {
        return axum::Json(SmartAddCreateResponse {
            key: None,
            error: Some("Invalid filename".to_string()),
        })
        .into_response();
    }

    let file_path = state.notes_dir.join(filename);

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

    // Build frontmatter
    let today = Utc::now().format("%Y-%m-%d");
    let mut frontmatter = format!(
        "---\ntitle: {}\ndate: {}\ntype: paper\nbib_key: {}\n",
        body.title, today, body.bib_key
    );

    if let Some(ref authors) = body.authors {
        if !authors.is_empty() {
            frontmatter.push_str(&format!("authors: {}\n", authors));
        }
    }
    if let Some(year) = body.year {
        frontmatter.push_str(&format!("year: {}\n", year));
    }
    if let Some(ref venue) = body.venue {
        if !venue.is_empty() {
            frontmatter.push_str(&format!("venue: {}\n", venue));
        }
    }
    if let Some(ref bibtex) = body.bibtex {
        if !bibtex.is_empty() {
            frontmatter.push_str("bibtex: |\n");
            for line in bibtex.lines() {
                frontmatter.push_str(&format!("  {}\n", line));
            }
        }
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
    if !is_logged_in(&jar) {
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
