//! Citation scanning: extract references from PDFs, match against note pool,
//! cache results in sled, and write `[@key]` links into managed markdown sections.

use crate::auth::is_logged_in;
use crate::models::{
    CitationMatch, CitationScanAllResult, CitationScanRequest, CitationScanResult,
    CitationWriteRequest, ExtractedReference, Note, NoteType,
};
use crate::notes::{normalize_title, parse_bibtex};
use crate::smart_add::{extract_arxiv_id, extract_doi};
use crate::AppState;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

#[cfg(test)]
#[path = "citations_test.rs"]
mod citations_test;

const CITATIONS_TREE: &str = "citations";
const BEGIN_MARKER: &str = "<!-- BEGIN AUTO-CITATIONS -->";
const END_MARKER: &str = "<!-- END AUTO-CITATIONS -->";

// ============================================================================
// PDF Text Extraction
// ============================================================================

/// Run `pdftotext` in a given mode and return stdout as a String.
fn run_pdftotext(path: &Path, layout: bool) -> Result<String, String> {
    let mut cmd = Command::new("pdftotext");
    if layout {
        cmd.arg("-layout");
    }
    cmd.arg(path.as_os_str()).arg("-");

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run pdftotext: {}. Is poppler installed?", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pdftotext failed: {}", stderr));
    }

    String::from_utf8(output.stdout)
        .map_err(|e| format!("pdftotext output not valid UTF-8: {}", e))
}

/// Extract text using the `pdf-extract` Rust crate (no external dependency).
fn run_pdf_extract(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Cannot read PDF: {}", e))?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("pdf-extract failed: {}", e))
}

/// Extract PDF text, trying multiple methods and picking whichever yields
/// the most reference entries:
/// - `pdftotext` without `-layout`: good for multi-column heading detection
/// - `pdftotext -layout`: good for preserving numbered reference formatting
/// - `pdf-extract` (native Rust): handles some PDFs that pdftotext misses
fn extract_pdf_text_best(path: &Path) -> Result<(String, Vec<String>), String> {
    let mut candidates: Vec<(String, Vec<String>)> = Vec::new();

    // Try pdftotext plain
    if let Ok(text) = run_pdftotext(path, false) {
        let refs = extract_references_from_text(&text);
        candidates.push((text, refs));
    }

    // Try pdftotext with layout
    if let Ok(text) = run_pdftotext(path, true) {
        let refs = extract_references_from_text(&text);
        candidates.push((text, refs));
    }

    // Try native Rust pdf-extract
    if let Ok(text) = run_pdf_extract(path) {
        let refs = extract_references_from_text(&text);
        candidates.push((text, refs));
    }

    if candidates.is_empty() {
        return Err("All PDF text extraction methods failed".to_string());
    }

    // Pick whichever yields the most references
    candidates.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    Ok(candidates.into_iter().next().unwrap())
}

// ============================================================================
// Reference Section Parsing
// ============================================================================

/// Find the reference/bibliography section at the end of extracted PDF text
/// and split it into individual reference entries.
fn extract_references_from_text(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();

    // Search backwards for a heading line that signals references.
    // Matches optional section numbering: "7. REFERENCES", "VII. REFERENCES",
    // "A. References", "REFERENCES", etc.
    let section_prefix = r"(?:\d+[\.\)]\s*|[IVXLC]+[\.\)]\s*|[A-Z][\.\)]\s*)?";
    let headings = r"references|bibliography|works cited|cited references|references and notes|literature cited|literature";
    // Strict match: heading alone on the line.
    let heading_strict = Regex::new(&format!(
        r"(?i)^\s*{section_prefix}({headings})\s*$"
    ))
    .unwrap();
    // Lenient match: heading at start of line (for multi-column PDFs where
    // column text may follow on the same line).
    let heading_lenient = Regex::new(&format!(
        r"(?i)^\s*{section_prefix}({headings})\s"
    ))
    .unwrap();

    let mut ref_start = None;
    // Search from the last 40% of the document
    let search_from = lines.len().saturating_sub(lines.len() * 2 / 5);
    for i in (search_from..lines.len()).rev() {
        if heading_strict.is_match(lines[i]) {
            ref_start = Some(i + 1);
            break;
        }
    }
    // Fallback: try lenient match
    if ref_start.is_none() {
        for i in (search_from..lines.len()).rev() {
            if heading_lenient.is_match(lines[i]) {
                ref_start = Some(i + 1);
                break;
            }
        }
    }

    // Fallback: if no heading found, look for a cluster of DOIs/URLs in the
    // last 30% — that's likely the reference section even without a heading.
    if ref_start.is_none() {
        let doi_re = Regex::new(r"10\.\d{4,}/|arxiv\.org/|doi\.org/|https?://").unwrap();
        let last_30_start = lines.len().saturating_sub(lines.len() * 3 / 10);
        // Count DOI/URL lines in sliding windows of 20 lines
        let window = 20;
        let mut best_density = 0usize;
        let mut best_start = 0usize;
        for start in last_30_start..lines.len().saturating_sub(window) {
            let count = lines[start..start + window]
                .iter()
                .filter(|l| doi_re.is_match(l))
                .count();
            if count > best_density && count >= 3 {
                best_density = count;
                best_start = start;
            }
        }
        if best_density >= 3 {
            // Walk backwards from the dense region to find the start of the first reference
            let bracket_re = Regex::new(r"^\s*\[\d+\]").unwrap();
            let mut scan = best_start;
            while scan > last_30_start {
                if lines[scan].trim().is_empty()
                    && scan + 1 < lines.len()
                    && bracket_re.is_match(lines[scan + 1])
                {
                    ref_start = Some(scan + 1);
                    break;
                }
                scan -= 1;
            }
            if ref_start.is_none() {
                ref_start = Some(best_start);
            }
        }
    }

    let ref_start = match ref_start {
        Some(s) => s,
        None => return Vec::new(),
    };

    let ref_lines = &lines[ref_start..];

    // --- Pre-processing: strip page headers/footers/noise ---
    // Many PDFs inject running headers, page numbers, and conference/journal
    // names between references. We detect and remove these lines.
    let cleaned_lines = strip_page_noise(ref_lines);
    let ref_text: String = cleaned_lines.join("\n");

    // Try numbered references in order of specificity:
    // 1. Bracketed: [1], [2], ...
    let numbered_bracket = Regex::new(r"(?m)^\s*\[(\d+)\]").unwrap();
    if numbered_bracket.find_count(&ref_text) >= 3 {
        return split_by_pattern(&ref_text, &numbered_bracket);
    }

    // 2. Dot-numbered: "1. AuthorName..." or "1. A. Author..."
    let numbered_dot = Regex::new(r"(?m)^\s*(\d+)\.\s+[A-Z]").unwrap();
    if numbered_dot.find_count(&ref_text) >= 3 {
        return split_by_pattern(&ref_text, &numbered_dot);
    }

    // 3. Bare number followed by spaces then author name on same line (layout mode):
    //    "  1    Serge Abiteboul..." or "10   Martin Bravenboer..."
    let numbered_space = Regex::new(r"(?m)^\s*(\d+)\s{2,}[A-Z]").unwrap();
    if numbered_space.find_count(&ref_text) >= 3 {
        return split_by_pattern(&ref_text, &numbered_space);
    }

    // 4. Bare numbers on their own line: common in some styles
    let bare_number = Regex::new(r"(?m)^\s*(\d+)\s*$").unwrap();
    let bare_count = bare_number.find_count(&ref_text);
    if bare_count >= 3 {
        let merged = merge_bare_numbers(&ref_text, &bare_number);
        let split_pat = Regex::new(r"(?m)^REFENTRY_\d+\s").unwrap();
        if split_pat.find_count(&merged) >= 3 {
            return split_by_pattern(&merged, &split_pat)
                .into_iter()
                .map(|s| {
                    Regex::new(r"^REFENTRY_\d+\s*")
                        .unwrap()
                        .replace(&s, "")
                        .to_string()
                        .trim()
                        .to_string()
                })
                .filter(|s| s.len() > 20)
                .collect();
        }
    }

    // 5. Author-year format (ACM style): "AuthorName. Year." or "AuthorName, ... Year."
    //    Each new reference starts at the beginning of a line with a capitalized name
    //    (FirstName LastName or FirstName particle LastName) followed by more names or a year.
    let author_year_start = Regex::new(
        r"(?m)^[A-Z][a-z]{1,20}[\s,]+(?:(?:de|van|von|le|la|di|del|den|der)\s+)?[A-Z][a-z]"
    ).unwrap();
    let author_year_count = author_year_start.find_count(&ref_text);
    if author_year_count >= 5 {
        let entries = split_by_pattern(&ref_text, &author_year_start);
        // Verify these look like real references (most should contain a year)
        let year_re = Regex::new(r"\b(?:19|20)\d{2}\b").unwrap();
        let entries_with_years = entries.iter().filter(|e| year_re.is_match(e)).count();
        if entries_with_years * 2 >= entries.len() {
            return entries;
        }
    }

    // 6. Fallback: split by blank-line separated blocks
    split_by_blank_lines(&ref_text)
}

/// Strip page headers, footers, page numbers, and running titles from reference lines.
/// These are common noise in multi-page reference sections.
fn strip_page_noise(lines: &[&str]) -> Vec<String> {
    // Detect repeated short lines (headers/footers appear on multiple pages)
    let mut short_line_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for line in lines {
        let trimmed = line.trim();
        // Short lines that look like noise: page numbers, headers
        if !trimmed.is_empty() && trimmed.len() < 60 {
            let normalized = trimmed.to_lowercase();
            *short_line_counts.entry(normalized).or_insert(0) += 1;
        }
    }

    // Lines that appear 2+ times and are short are likely headers/footers
    let repeated_noise: std::collections::HashSet<String> = short_line_counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(line, _)| line)
        .collect();

    // Also detect standalone page numbers and very short noise lines
    let page_num_re = Regex::new(r"^\s*\d{1,4}\s*$").unwrap();
    // Detect page:column markers like "7:27", "7:28"
    let page_col_re = Regex::new(r"^\s*\d+:\d+\s*$").unwrap();

    lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return true; // Keep blank lines for structure
            }
            // Filter lines containing form feed characters (page breaks from pdftotext)
            if line.contains('\x0c') {
                return false;
            }
            // Filter out repeated noise (running headers, conference names)
            if repeated_noise.contains(&trimmed.to_lowercase()) && trimmed.len() < 60 {
                // But don't filter if it looks like a real reference number
                if page_num_re.is_match(trimmed) {
                    // Bare numbers might be reference numbers — keep if ≤ 200
                    let num: usize = trimmed.trim().parse().unwrap_or(999);
                    return num <= 200;
                }
                return false;
            }
            // Filter standalone page numbers that appear after form feeds
            // (pdftotext sometimes puts page number on its own line)
            if page_num_re.is_match(trimmed) {
                let num: usize = trimmed.trim().parse().unwrap_or(999);
                // Reference numbers are typically sequential and < 200;
                // but isolated page numbers > 10 in a reference section
                // (where we expect [1]-style refs) are likely noise
                if num > 200 {
                    return false;
                }
            }
            // Filter standalone page:column markers
            if page_col_re.is_match(trimmed) {
                return false;
            }
            true
        })
        .map(|s| s.to_string())
        .collect()
}

/// Merge bare reference numbers (alone on a line) with the text that follows,
/// producing lines like "REFENTRY_1 Author Name, ..."
fn merge_bare_numbers(text: &str, bare_pattern: &Regex) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if let Some(caps) = bare_pattern.captures(trimmed) {
            let num = &caps[1];
            // Skip blank lines after the number to find the actual reference text
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            // Collect text until next bare number or end
            let mut entry_lines = vec![format!("REFENTRY_{} ", num)];
            while j < lines.len() {
                let next_trimmed = lines[j].trim();
                if bare_pattern.is_match(next_trimmed) {
                    break;
                }
                if !next_trimmed.is_empty() {
                    entry_lines.push(next_trimmed.to_string());
                }
                j += 1;
            }
            result.push(entry_lines.join(" "));
            i = j;
        } else {
            // Non-number line outside a reference — skip (likely noise before first ref)
            i += 1;
        }
    }
    result.join("\n")
}

/// Split reference text at each match of `pattern`, collecting everything between
/// successive matches as one entry.
fn split_by_pattern(text: &str, pattern: &Regex) -> Vec<String> {
    let mut entries = Vec::new();
    let match_positions: Vec<usize> = pattern.find_iter(text).map(|m| m.start()).collect();

    for (i, &start) in match_positions.iter().enumerate() {
        let end = if i + 1 < match_positions.len() {
            match_positions[i + 1]
        } else {
            text.len()
        };
        let entry = text[start..end].trim().to_string();
        if !entry.is_empty() {
            entries.push(entry);
        }
    }
    entries
}

/// Split by blank lines (two or more consecutive newlines).
/// Merges short fragments (< 60 chars) into the preceding entry since
/// they're usually continuations that got split by a stray blank line.
fn split_by_blank_lines(text: &str) -> Vec<String> {
    let splitter = Regex::new(r"\n\s*\n").unwrap();
    let raw: Vec<String> = splitter
        .split(text)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() > 40) // Skip very short fragments
        .collect();

    // Merge short fragments into the previous entry
    let mut merged: Vec<String> = Vec::new();
    for entry in raw {
        if entry.len() < 60 && !merged.is_empty() {
            // Append to previous — likely a continuation
            let last = merged.last_mut().unwrap();
            last.push(' ');
            last.push_str(&entry);
        } else {
            merged.push(entry);
        }
    }
    merged
}

/// Helper trait for counting regex matches without allocating a Vec.
trait FindCount {
    fn find_count(&self, text: &str) -> usize;
}
impl FindCount for Regex {
    fn find_count(&self, text: &str) -> usize {
        self.find_iter(text).count()
    }
}

// ============================================================================
// Reference Text Parsing
// ============================================================================

/// Parse a single reference entry text to extract structured identifiers.
fn parse_reference_text(text: &str, index: usize) -> ExtractedReference {
    let doi = extract_doi(text);
    let arxiv_id = extract_arxiv_id(text);

    // Extract year: look for 4-digit years between 1900-2099
    let year_re = Regex::new(r"\b((?:19|20)\d{2})\b").unwrap();
    let year = year_re
        .captures_iter(text)
        .filter_map(|c| c.get(1)?.as_str().parse::<i32>().ok())
        .last(); // Prefer later occurrences (often the publication year)

    // Extract author last names: typically at the start before the year
    let authors = extract_author_lastnames(text);

    // Extract title: heuristic — text between first period and second period,
    // or between first comma group and the venue/year.
    let title = extract_title_from_ref(text);

    ExtractedReference {
        raw_text: text.to_string(),
        index,
        doi,
        arxiv_id,
        title,
        authors,
        year,
    }
}

/// Extract author last names from the beginning of a reference entry.
fn extract_author_lastnames(text: &str) -> Vec<String> {
    // Strip leading [N] or N. numbering
    let stripped = Regex::new(r"^\s*(?:\[\d+\]\s*|\d+\.\s*)")
        .unwrap()
        .replace(text, "");

    // Take text up to first year or title delimiter
    let cutoff_re = Regex::new(r#"(?:19|20)\d{2}|["\u{201c}\u{201d}]"#).unwrap();
    let author_part = match cutoff_re.find(&stripped) {
        Some(m) => &stripped[..m.start()],
        None => {
            // Take up to first period followed by a space and uppercase
            let period_re = Regex::new(r"\.\s+[A-Z][a-z]").unwrap();
            match period_re.find(&stripped) {
                Some(m) => &stripped[..m.start() + 1],
                None => {
                    let len = stripped.len().min(200);
                    &stripped[..len]
                }
            }
        }
    };

    // Handle "et al." — only use text before it
    let author_text = if let Some(pos) = author_part.find("et al") {
        &author_part[..pos]
    } else {
        author_part
    };

    // Try explicit "LastName, F." and "F. LastName" patterns first
    let explicit_re = Regex::new(r"\b([A-Z][a-z]{1,20}(?:-[A-Z][a-z]{1,20})?)\s*,\s*[A-Z]\.").unwrap();
    let mut names: Vec<String> = explicit_re
        .captures_iter(author_text)
        .filter_map(|c| {
            let name = c.get(1)?.as_str();
            if is_author_skip_word(name) { None } else { Some(name.to_lowercase()) }
        })
        .collect();

    // If explicit patterns didn't find enough, fall back to capitalized word extraction
    if names.len() < 2 {
        let name_re = Regex::new(r"\b([A-Z][a-z]{1,20}(?:-[A-Z][a-z]{1,20})?)\b").unwrap();
        names = name_re
            .captures_iter(author_text)
            .filter_map(|c| {
                let name = c.get(1)?.as_str();
                if is_author_skip_word(name) { None } else { Some(name.to_lowercase()) }
            })
            .collect();
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    names.retain(|n| seen.insert(n.clone()));
    names
}

/// Words that look like capitalized names but are actually venue/method terms.
fn is_author_skip_word(name: &str) -> bool {
    const SKIP: &[&str] = &[
        "The", "And", "For", "With", "From", "This", "That", "Into", "Over", "Under",
        "Vol", "Proc", "IEEE", "ACM", "Int", "Conf", "Proceedings", "Conference",
        "International", "Workshop", "Journal", "University", "Department",
        "Technical", "Report", "Available", "Accessed", "Retrieved",
        "Lecture", "Notes", "Computer", "Science", "Society", "Press",
        "Springer", "Chapter", "Section", "Part", "New", "York",
    ];
    SKIP.contains(&name)
}

/// Venue/source indicators that signal the end of a title in a reference.
const VENUE_INDICATORS: &[&str] = &[
    "In ", "In:", "Proceedings", "Proc.", "Journal of", "Trans.",
    "IEEE ", "ACM ", "SIGMOD", "VLDB", "ICSE", "PLDI", "POPL",
    "ICFP", "OOPSLA", "ECOOP", "SAS ", "CAV ", "LICS", "ICALP",
    "Springer", "Lecture Notes", "LNCS", "arXiv:", "https://", "http://",
    "pp.", "vol.", "Vol.", "pages ", "Technical Report",
    "Ph.D.", "PhD", "Master", "Dissertation", "thesis",
];

/// Heuristic title extraction from a reference entry.
/// Tries multiple strategies in order, returning the first success.
fn extract_title_from_ref(text: &str) -> Option<String> {
    // Strip leading numbering: [1], 1., etc.
    let stripped = Regex::new(r"^\s*(?:\[\d+\]\s*|\d+\.\s*)")
        .unwrap()
        .replace(text, "")
        .to_string();

    // Strategy 1: Quoted title (works for styles that quote titles)
    let quoted_re = Regex::new(r#"["\u{201c}]([^"\u{201d}]{10,}?)["\u{201d}]"#).unwrap();
    if let Some(caps) = quoted_re.captures(&stripped) {
        if let Some(m) = caps.get(1) {
            return Some(m.as_str().trim().to_string());
        }
    }

    // Strategy 2: Post-author, pre-venue extraction
    // Find the title between the author block and the first venue indicator.
    // Author block ends at the year or the first sentence-ending period.
    if let Some(title) = extract_title_post_author(&stripped) {
        if title.len() >= 10 {
            return Some(title);
        }
    }

    // Strategy 3: Longest capitalized phrase
    // Find the longest contiguous run of Title Case words (min 4 words).
    if let Some(title) = extract_longest_title_case_phrase(&stripped) {
        return Some(title);
    }

    None
}

/// Strategy 2: Extract title from the region between authors and venue.
fn extract_title_post_author(text: &str) -> Option<String> {
    // Find the end of the author block.
    // Common patterns: "Author, A. and Author, B. (2020)." or "Author, A., Author, B. 2020."
    // The author block typically ends at the year or after a period following initials.

    // Find the first year occurrence
    let year_re = Regex::new(r"\b((?:19|20)\d{2})\b").unwrap();
    let year_match = year_re.find(text);

    // Find the start of the title region: just after the year (and any trailing punctuation),
    // or after the first period that follows a sequence of short name-like tokens.
    let title_start = if let Some(ym) = year_match {
        // Skip past year and trailing ". " or ") " or ". "
        let after_year = &text[ym.end()..];
        let skip = after_year
            .chars()
            .take_while(|c| *c == '.' || *c == ')' || *c == ',' || *c == ' ' || *c == ']')
            .count();
        ym.end() + skip
    } else {
        // No year found — try to find end of author block by looking for
        // a period followed by a space and an uppercase letter (sentence boundary)
        let period_re = Regex::new(r"\.\s+[A-Z][a-z]").unwrap();
        match period_re.find(text) {
            Some(m) => m.start() + 2, // skip the ". "
            None => return None,
        }
    };

    if title_start >= text.len() {
        return None;
    }

    let rest = &text[title_start..];

    // Find where the title ends: at the first venue indicator or sentence-ending period
    // followed by a venue-like context.
    let mut title_end = rest.len();

    // Check for venue indicators
    for indicator in VENUE_INDICATORS {
        if let Some(pos) = rest.find(indicator) {
            if pos > 0 && pos < title_end {
                title_end = pos;
            }
        }
    }

    // Also look for a period followed by a space and then a venue-like word
    let period_venue_re = Regex::new(
        r"\.\s+(?:In\b|Proceedings|Proc\.|Journal|Trans\.|IEEE|ACM|Springer|Lecture|LNCS|Ph\.D|vol\.|pp\.|Technical|\d{4})"
    ).unwrap();
    if let Some(m) = period_venue_re.find(rest) {
        if m.start() < title_end {
            title_end = m.start();
        }
    }

    let candidate = rest[..title_end].trim().trim_end_matches('.');
    if candidate.len() >= 10 {
        Some(candidate.to_string())
    } else {
        None
    }
}

/// Strategy 3: Find the longest Title Case phrase (most words capitalized).
fn extract_longest_title_case_phrase(text: &str) -> Option<String> {
    // Split into sentences at period+space boundaries
    let sentences: Vec<&str> = text.split(". ").collect();
    let mut best: Option<String> = None;
    let mut best_len = 0;

    for sentence in &sentences {
        let words: Vec<&str> = sentence.split_whitespace().collect();
        if words.len() < 4 {
            continue;
        }

        // Count capitalized words (excluding small words like "of", "the", "and")
        let small_words = ["of", "the", "and", "in", "on", "for", "a", "an", "to",
                           "with", "by", "from", "or", "as", "at", "its", "via", "vs"];
        let cap_count = words.iter().filter(|w| {
            let first = w.chars().next().unwrap_or('a');
            first.is_uppercase() || small_words.contains(&w.to_lowercase().as_str())
        }).count();

        // At least 60% of words should be capitalized or small-words
        if cap_count * 100 / words.len() >= 60 && words.len() > best_len {
            // Skip if it looks like an author block (many single-letter initials)
            let initial_count = words.iter().filter(|w| {
                w.len() <= 2 && w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            }).count();
            if initial_count * 3 > words.len() {
                continue; // Too many initials — probably authors
            }

            best_len = words.len();
            best = Some(sentence.trim().trim_end_matches('.').to_string());
        }
    }

    // Only return if we found something substantial (at least 4 words)
    best.filter(|s| s.split_whitespace().count() >= 4)
}

// ============================================================================
// Note Pool Index for Matching
// ============================================================================

pub struct NotePoolIndex {
    doi_to_key: HashMap<String, String>,
    arxiv_to_key: HashMap<String, String>,
    title_to_key: HashMap<String, String>,
    /// (normalized_title, note_key) pairs for fuzzy title matching
    title_entries: Vec<(String, String)>,
    /// author_lastname + year -> list of keys (may be ambiguous)
    author_year_to_keys: HashMap<String, Vec<String>>,
}

/// Compute Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    let m = a_bytes.len();
    let n = b_bytes.len();

    // Short-circuit: if length difference alone exceeds any reasonable threshold, bail
    if m.abs_diff(n) > m.max(n) / 5 + 3 {
        return m.max(n);
    }

    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];

    for j in 0..=n {
        prev[j] = j;
    }

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_bytes[i - 1] == b_bytes[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

impl NotePoolIndex {
    /// Build lookup maps from all notes in the pool.
    pub fn build(notes: &[Note]) -> Self {
        use rayon::prelude::*;

        let name_re = Regex::new(r"\b([A-Z][a-z]{1,20})\b").unwrap();

        // Parallel per-note extraction
        let per_note: Vec<_> = notes
            .par_iter()
            .map(|note| {
                let key = &note.key;
                let mut dois = Vec::new();
                let mut arxivs = Vec::new();
                let mut title_entry = None;
                let mut author_years = Vec::new();

                if let NoteType::Paper(ref meta) = note.note_type {
                    for source in &meta.sources {
                        match source.source_type.as_str() {
                            "doi" => dois.push((source.identifier.to_lowercase(), key.clone())),
                            "arxiv" => arxivs.push((source.identifier.clone(), key.clone())),
                            _ => {}
                        }
                    }

                    for bib_entry in &meta.bibtex_entries {
                        if let Some(parsed) = parse_bibtex(bib_entry) {
                            if let Some(ref doi) = parsed.doi {
                                dois.push((doi.to_lowercase(), key.clone()));
                            }
                            if let Some(ref eprint) = parsed.eprint {
                                arxivs.push((eprint.clone(), key.clone()));
                            }
                            if let (Some(ref authors), Some(year)) =
                                (&parsed.author, parsed.year)
                            {
                                for cap in name_re.captures_iter(authors) {
                                    if let Some(name) = cap.get(1) {
                                        let lookup =
                                            format!("{}_{}", name.as_str().to_lowercase(), year);
                                        author_years.push((lookup, key.clone()));
                                    }
                                }
                            }
                        }
                    }
                }

                let norm = normalize_title(&note.title);
                if norm.len() >= 5 {
                    title_entry = Some((norm, key.clone()));
                }

                (dois, arxivs, title_entry, author_years)
            })
            .collect();

        // Sequential merge
        let mut doi_to_key = HashMap::new();
        let mut arxiv_to_key = HashMap::new();
        let mut title_to_key = HashMap::new();
        let mut title_entries = Vec::new();
        let mut author_year_to_keys: HashMap<String, Vec<String>> = HashMap::new();

        for (dois, arxivs, title_entry, author_years) in per_note {
            for (doi, key) in dois {
                doi_to_key.entry(doi).or_insert(key);
            }
            for (arxiv, key) in arxivs {
                arxiv_to_key.entry(arxiv).or_insert(key);
            }
            if let Some((norm, key)) = title_entry {
                title_to_key.insert(norm.clone(), key.clone());
                title_entries.push((norm, key));
            }
            for (lookup, key) in author_years {
                author_year_to_keys.entry(lookup).or_default().push(key);
            }
        }

        // Deduplicate author_year entries
        for keys in author_year_to_keys.values_mut() {
            keys.sort();
            keys.dedup();
        }

        NotePoolIndex {
            doi_to_key,
            arxiv_to_key,
            title_to_key,
            title_entries,
            author_year_to_keys,
        }
    }

    /// Match a parsed reference against the note pool using tiered matching.
    pub fn match_reference(&self, reference: &ExtractedReference) -> Option<CitationMatch> {
        // Tier 1: DOI exact match
        if let Some(ref doi) = reference.doi {
            if let Some(key) = self.doi_to_key.get(&doi.to_lowercase()) {
                return Some(CitationMatch {
                    target_key: key.clone(),
                    match_type: "doi".to_string(),
                    confidence: 1.0,
                    raw_text: reference.raw_text.clone(),
                });
            }
        }

        // Tier 2: arXiv exact match
        if let Some(ref arxiv_id) = reference.arxiv_id {
            if let Some(key) = self.arxiv_to_key.get(arxiv_id) {
                return Some(CitationMatch {
                    target_key: key.clone(),
                    match_type: "arxiv".to_string(),
                    confidence: 0.95,
                    raw_text: reference.raw_text.clone(),
                });
            }
        }

        // Tier 3: Title matching — exact normalized first, then edit-distance fallback
        if let Some(ref title) = reference.title {
            let norm = normalize_title(title);
            if norm.len() >= 5 {
                // 3a: Exact normalized match (fast HashMap lookup)
                if let Some(key) = self.title_to_key.get(&norm) {
                    return Some(CitationMatch {
                        target_key: key.clone(),
                        match_type: "title".to_string(),
                        confidence: 0.90,
                        raw_text: reference.raw_text.clone(),
                    });
                }

                // 3b: Fuzzy match — Levenshtein edit distance ≤ 5% of title length
                // Linear scan over all titles (hundreds, not millions, so fine)
                let max_dist = (norm.len() / 20).max(2); // ≤5% of chars, minimum 2
                let mut best_match: Option<(&str, usize)> = None;

                for (pool_title, pool_key) in &self.title_entries {
                    // Quick length filter before computing edit distance
                    if pool_title.len().abs_diff(norm.len()) > max_dist {
                        continue;
                    }
                    let dist = edit_distance(&norm, pool_title);
                    if dist <= max_dist && dist > 0 {
                        if best_match.is_none() || dist < best_match.unwrap().1 {
                            best_match = Some((pool_key.as_str(), dist));
                        }
                    }
                }

                if let Some((key, dist)) = best_match {
                    // Confidence scales from 0.85 (at max_dist) to 0.89 (at dist=1)
                    let confidence = 0.90 - (dist as f64 * 0.05 / max_dist as f64).min(0.10);
                    return Some(CitationMatch {
                        target_key: key.to_string(),
                        match_type: "title_fuzzy".to_string(),
                        confidence,
                        raw_text: reference.raw_text.clone(),
                    });
                }
            }
        }

        // Tier 4: Author last name + year — vote counting across all extracted authors
        if let Some(year) = reference.year {
            let mut candidate_votes: HashMap<&str, usize> = HashMap::new();

            for author in &reference.authors {
                let lookup = format!("{}_{}", author, year);
                if let Some(keys) = self.author_year_to_keys.get(&lookup) {
                    if keys.len() == 1 {
                        *candidate_votes.entry(&keys[0]).or_insert(0) += 1;
                    }
                }
            }

            // Pick candidate with most author matches
            if let Some((&best_key, &vote_count)) =
                candidate_votes.iter().max_by_key(|(_, &v)| v)
            {
                // Require either 2+ author matches, or 1 match on the first author
                let is_first_author_match = reference.authors.first().map_or(false, |first| {
                    let lookup = format!("{}_{}", first, year);
                    self.author_year_to_keys
                        .get(&lookup)
                        .map_or(false, |keys| keys.len() == 1 && keys[0] == best_key)
                });

                if vote_count >= 2 || (vote_count == 1 && is_first_author_match) {
                    let confidence = match vote_count {
                        1 => 0.40,
                        2 => 0.55,
                        _ => 0.65, // 3+
                    };
                    return Some(CitationMatch {
                        target_key: best_key.to_string(),
                        match_type: "author_year".to_string(),
                        confidence,
                        raw_text: reference.raw_text.clone(),
                    });
                }
            }
        }

        None
    }
}

// ============================================================================
// PDF Hashing for Cache Validation
// ============================================================================

/// SHA256 of the first 64KB of a file, sufficient for change detection.
fn hash_pdf(path: &Path) -> Result<String, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("Cannot open PDF: {}", e))?;
    let mut buf = vec![0u8; 65536];
    let n = file
        .read(&mut buf)
        .map_err(|e| format!("Cannot read PDF: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&buf[..n]);
    Ok(format!("{:x}", hasher.finalize()))
}

// ============================================================================
// Sled Cache Operations
// ============================================================================

fn load_cached_result(db: &sled::Db, key: &str) -> Option<CitationScanResult> {
    let tree = db.open_tree(CITATIONS_TREE).ok()?;
    let data = tree.get(key.as_bytes()).ok()??;
    serde_json::from_slice(&data).ok()
}

fn save_cached_result(db: &sled::Db, result: &CitationScanResult) -> Result<(), String> {
    let tree = db
        .open_tree(CITATIONS_TREE)
        .map_err(|e| format!("Cannot open citations tree: {}", e))?;
    let json = serde_json::to_vec(result).map_err(|e| format!("JSON serialize error: {}", e))?;
    tree.insert(result.source_key.as_bytes(), json)
        .map_err(|e| format!("Sled insert error: {}", e))?;
    Ok(())
}

// ============================================================================
// Core Scan Logic
// ============================================================================

/// Core scan logic that uses a pre-built NotePoolIndex.
fn scan_note_pdf_with_index(
    note: &Note,
    index: &NotePoolIndex,
    pdfs_dir: &Path,
    db: &sled::Db,
    force: bool,
) -> Result<CitationScanResult, String> {
    let pdf_filename = note
        .pdf
        .as_deref()
        .ok_or_else(|| "Note has no attached PDF".to_string())?;

    let pdf_path = pdfs_dir.join(pdf_filename);
    if !pdf_path.exists() {
        return Err(format!("PDF file not found: {}", pdf_filename));
    }

    let current_hash = hash_pdf(&pdf_path)?;

    // Check cache
    if !force {
        if let Some(cached) = load_cached_result(db, &note.key) {
            if cached.pdf_hash == current_hash {
                return Ok(cached);
            }
        }
    }

    // Extract text — tries both with and without -layout, picks whichever yields more refs
    let (_text, raw_refs) = extract_pdf_text_best(&pdf_path)?;
    let parsed_refs: Vec<ExtractedReference> = raw_refs
        .iter()
        .enumerate()
        .map(|(i, r)| parse_reference_text(r, i))
        .collect();

    let mut matches = Vec::new();
    let mut matched_keys = std::collections::HashSet::new();
    let mut unmatched = 0;

    for parsed in &parsed_refs {
        if let Some(m) = index.match_reference(parsed) {
            // Don't match self and avoid duplicates
            if m.target_key != note.key && !matched_keys.contains(&m.target_key) {
                matched_keys.insert(m.target_key.clone());
                matches.push(m);
            }
        } else {
            unmatched += 1;
        }
    }

    let result = CitationScanResult {
        source_key: note.key.clone(),
        matches,
        unmatched_count: unmatched,
        timestamp: Utc::now().to_rfc3339(),
        pdf_hash: current_hash,
    };

    save_cached_result(db, &result)?;

    Ok(result)
}

fn scan_note_pdf(
    note: &Note,
    notes: &[Note],
    pdfs_dir: &Path,
    db: &sled::Db,
    force: bool,
) -> Result<CitationScanResult, String> {
    let index = NotePoolIndex::build(notes);
    scan_note_pdf_with_index(note, &index, pdfs_dir, db, force)
}

// ============================================================================
// Markdown Writing
// ============================================================================

/// Write citation matches as a `## References` section with `[@key]` links,
/// wrapped in managed markers for idempotent updates.
fn write_citations_to_markdown(
    note: &Note,
    result: &CitationScanResult,
    notes_map: &HashMap<String, Note>,
    notes_dir: &Path,
) -> Result<(), String> {
    let content = &note.full_file_content;

    // Build the new auto-citations block
    let mut block = String::new();
    block.push_str(BEGIN_MARKER);
    block.push('\n');
    block.push_str("## References\n\n");

    let mut sorted_matches = result.matches.clone();
    sorted_matches.sort_by(|a, b| a.target_key.cmp(&b.target_key));

    for m in &sorted_matches {
        let title = notes_map
            .get(&m.target_key)
            .map(|n| n.title.as_str())
            .unwrap_or("Unknown");
        block.push_str(&format!("- [@{}] {}\n", m.target_key, title));
    }
    block.push_str(END_MARKER);

    // Replace existing block or append
    let new_content = if let Some(begin_pos) = content.find(BEGIN_MARKER) {
        if let Some(end_pos) = content.find(END_MARKER) {
            let end = end_pos + END_MARKER.len();
            format!("{}{}{}", &content[..begin_pos], block, &content[end..])
        } else {
            // Malformed: replace from begin marker to end of file
            format!("{}{}", &content[..begin_pos], block)
        }
    } else {
        // Append with blank line
        let trimmed = content.trim_end();
        format!("{}\n\n{}\n", trimmed, block)
    };

    let full_path = notes_dir.join(&note.path);
    std::fs::write(&full_path, &new_content)
        .map_err(|e| format!("Failed to write note to {}: {}", full_path.display(), e))?;

    Ok(())
}

// ============================================================================
// API Handlers
// ============================================================================

/// POST /api/citations/scan — scan one paper's PDF for citations
pub async fn citation_scan(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(req): axum::Json<CitationScanRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes = state.load_notes();
    let notes_map: HashMap<String, Note> = notes.iter().map(|n| (n.key.clone(), n.clone())).collect();

    let note = match notes_map.get(&req.note_key) {
        Some(n) => n.clone(),
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                format!("Note not found: {}", req.note_key),
            )
                .into_response();
        }
    };

    let pdfs_dir = state.pdfs_dir.clone();
    let db = state.db.clone();
    let force = req.force;

    // Run pdftotext on spawn_blocking to avoid blocking async runtime
    let result = tokio::task::spawn_blocking(move || {
        scan_note_pdf(&note, &notes, &pdfs_dir, &db, force)
    })
    .await;

    match result {
        Ok(Ok(scan_result)) => {
            // Sync citation edges into the graph index
            let _ = crate::graph_index::sync_citations(&state.db, &req.note_key);
            axum::Json(scan_result).into_response()
        }
        Ok(Err(e)) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task join error: {}", e),
        )
            .into_response(),
    }
}

/// POST /api/citations/write — write cached scan results into note's markdown
pub async fn citation_write(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    axum::Json(req): axum::Json<CitationWriteRequest>,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes_map = state.notes_map();

    let note = match notes_map.get(&req.note_key) {
        Some(n) => n.clone(),
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                format!("Note not found: {}", req.note_key),
            )
                .into_response();
        }
    };

    // Build the effective scan result: filter by accepted_keys if present
    let effective_result = if let Some(ref accepted_keys) = req.accepted_keys {
        if accepted_keys.is_empty() {
            return (axum::http::StatusCode::OK, "No citations to write").into_response();
        }

        let accepted_set: std::collections::HashSet<&str> =
            accepted_keys.iter().map(|k| k.as_str()).collect();

        // Start from cached scan results if available
        let cached_matches = load_cached_result(&state.db, &req.note_key)
            .map(|r| r.matches)
            .unwrap_or_default();

        // Keep only accepted matches from the scan
        let mut filtered: Vec<CitationMatch> = cached_matches
            .into_iter()
            .filter(|m| accepted_set.contains(m.target_key.as_str()))
            .collect();

        // Find keys that were manually added (in accepted_keys but not in scan results)
        let scan_keys: std::collections::HashSet<String> =
            filtered.iter().map(|m| m.target_key.clone()).collect();
        for key in accepted_keys {
            if !scan_keys.contains(key.as_str()) {
                // Manual addition — create a synthetic CitationMatch
                filtered.push(CitationMatch {
                    target_key: key.clone(),
                    match_type: "manual".to_string(),
                    confidence: 1.0,
                    raw_text: String::new(),
                });
            }
        }

        CitationScanResult {
            source_key: req.note_key.clone(),
            matches: filtered,
            unmatched_count: 0,
            timestamp: Utc::now().to_rfc3339(),
            pdf_hash: String::new(),
        }
    } else {
        // Legacy: write all cached matches
        match load_cached_result(&state.db, &req.note_key) {
            Some(r) if !r.matches.is_empty() => r,
            Some(_) => {
                return (axum::http::StatusCode::OK, "No matches to write").into_response();
            }
            None => {
                return (
                    axum::http::StatusCode::BAD_REQUEST,
                    "No cached scan results. Run scan first.",
                )
                    .into_response();
            }
        }
    };

    match write_citations_to_markdown(&note, &effective_result, &notes_map, &state.notes_dir) {
        Ok(()) => {
            state.invalidate_notes_cache();
            state.reindex_graph_note(&req.note_key);
            let msg = format!("Wrote {} citation(s) to {}", effective_result.matches.len(), req.note_key);
            (axum::http::StatusCode::OK, msg).into_response()
        }
        Err(e) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

/// POST /api/citations/scan-all — bulk scan all papers with PDFs
pub async fn citation_scan_all(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Response {
    if !is_logged_in(&jar, &state.db) {
        return (axum::http::StatusCode::UNAUTHORIZED, "Not logged in").into_response();
    }

    let notes = state.load_notes();
    let pdfs_dir = state.pdfs_dir.clone();
    let db = state.db.clone();

    let result = tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;

        let notes_with_pdf: Vec<&Note> = notes.iter().filter(|n| n.pdf.is_some()).collect();
        let skipped_no_pdf = notes.len() - notes_with_pdf.len();

        // Parallel cache check + hash computation
        let cache_status: Vec<(&Note, bool, usize)> = notes_with_pdf
            .par_iter()
            .map(|&note| {
                if let Some(cached) = load_cached_result(&db, &note.key) {
                    let pdf_path = pdfs_dir.join(note.pdf.as_deref().unwrap());
                    if let Ok(hash) = hash_pdf(&pdf_path) {
                        if cached.pdf_hash == hash {
                            return (note, true, cached.matches.len());
                        }
                    }
                }
                (note, false, 0)
            })
            .collect();

        let mut skipped_cached = 0;
        let mut cached_matches = 0;
        let mut to_scan = Vec::new();

        for (note, is_cached, match_count) in cache_status {
            if is_cached {
                skipped_cached += 1;
                cached_matches += match_count;
            } else {
                to_scan.push(note);
            }
        }

        // Build index once for all uncached scans
        let index = NotePoolIndex::build(&notes);

        // Parallel scan for uncached notes
        let scan_results: Vec<(String, Result<CitationScanResult, String>)> = to_scan
            .par_iter()
            .map(|note| {
                let result = scan_note_pdf_with_index(note, &index, &pdfs_dir, &db, false);
                (note.key.clone(), result)
            })
            .collect();

        let mut scanned = 0;
        let mut total_matches = cached_matches;
        let mut errors = Vec::new();

        for (key, result) in scan_results {
            match result {
                Ok(r) => {
                    total_matches += r.matches.len();
                    scanned += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", key, e));
                }
            }
        }

        CitationScanAllResult {
            scanned,
            skipped_cached,
            skipped_no_pdf,
            total_matches,
            errors,
        }
    })
    .await;

    match result {
        Ok(scan_all_result) => {
            // Sync all citation edges into the graph index
            let _ = crate::graph_index::sync_all_citations(&state.db);
            axum::Json(scan_all_result).into_response()
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Task join error: {}", e),
        )
            .into_response(),
    }
}
