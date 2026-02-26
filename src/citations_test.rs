//! Tests for the citation scanning pipeline.
//!
//! Tests operate on pre-extracted `.txt` fixtures (from pdftotext) so they're
//! deterministic and don't require pdftotext at test time.

use super::*;
use crate::models::{ExtractedReference, Note, NoteType, PaperMeta, PaperSource};
use chrono::Utc;
use std::path::PathBuf;

// ============================================================================
// Helpers
// ============================================================================

/// Build a minimal Note for testing with the given key, title, and optional
/// BibTeX / source identifiers.
fn mock_note(key: &str, title: &str, doi: Option<&str>, arxiv: Option<&str>, bibtex: Option<&str>) -> Note {
    let mut sources = Vec::new();
    if let Some(d) = doi {
        sources.push(PaperSource {
            source_type: "doi".to_string(),
            identifier: d.to_string(),
        });
    }
    if let Some(a) = arxiv {
        sources.push(PaperSource {
            source_type: "arxiv".to_string(),
            identifier: a.to_string(),
        });
    }
    let bibtex_entries = bibtex.map(|b| vec![b.to_string()]).unwrap_or_default();
    Note {
        key: key.to_string(),
        path: PathBuf::from(format!("{}.md", key)),
        title: title.to_string(),
        date: None,
        note_type: if bibtex.is_some() || doi.is_some() || arxiv.is_some() {
            NoteType::Paper(PaperMeta {
                bibtex_entries,
                canonical_key: None,
                sources,
            })
        } else {
            NoteType::Note
        },
        parent_key: None,
        time_entries: Vec::new(),
        raw_content: String::new(),
        full_file_content: String::new(),
        modified: Utc::now(),
        pdf: None,
        hidden: false,
    }
}

/// Load a test fixture file from tests/fixtures/.
fn load_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot load fixture {}: {}", path.display(), e))
}

/// Build a mock note pool matching papers commonly cited in PL/systems research.
fn build_test_pool() -> Vec<Note> {
    vec![
        mock_note("abi95", "Foundations of Databases", Some("10.5555/551350"), None,
            Some(r#"@book{abiteboul1995, author = {Serge Abiteboul and Richard Hull and Victor Vianu}, title = {Foundations of Databases}, year = {1995}, publisher = {Addison-Wesley}}"#)),
        mock_note("wha05", "Using Datalog with binary decision diagrams for program analysis", None, None,
            Some(r#"@inproceedings{whaley2005, author = {John Whaley and Dzintars Avots and Michael Carbin and Monica S. Lam}, title = {Using Datalog with binary decision diagrams for program analysis}, year = {2005}, booktitle = {APLAS}}"#)),
        mock_note("sma14", "Introspective Analysis: Context-Sensitivity, Across the Board", None, None,
            Some(r#"@inproceedings{smaragdakis2014, author = {Yannis Smaragdakis and George Kastrinis and George Balatsouras}, title = {Introspective analysis: context-sensitivity, across the board}, year = {2014}, booktitle = {PLDI}}"#)),
        mock_note("cou79", "Systematic design of program analysis frameworks", None, None,
            Some(r#"@inproceedings{cousot1979, author = {Patrick Cousot and Radhia Cousot}, title = {Systematic design of program analysis frameworks}, year = {1979}, booktitle = {POPL}}"#)),
        mock_note("mig10", "Resolving and exploiting the k-CFA paradox", None, None,
            Some(r#"@inproceedings{might2010, author = {Matthew Might and Yannis Smaragdakis and David Van Horn}, title = {Resolving and exploiting the k-CFA paradox: illuminating functional vs. object-oriented program analysis}, year = {2010}, booktitle = {PLDI}}"#)),
        mock_note("vh10", "Abstracting Abstract Machines", None, None,
            Some(r#"@inproceedings{vanhorn2010, author = {David Van Horn and Matthew Might}, title = {Abstracting abstract machines}, year = {2010}, booktitle = {ICFP}}"#)),
        mock_note("tat09", "Equality Saturation: A New Approach to Optimization", None, None,
            Some(r#"@inproceedings{tate2009, author = {Ross Tate and Michael Stepp and Zachary Tatlock and Sorin Lerner}, title = {Equality Saturation: A New Approach to Optimization}, year = {2009}, booktitle = {POPL}}"#)),
        mock_note("sch16", "On fast large-scale program analysis in datalog", None, None,
            Some(r#"@inproceedings{scholz2016, author = {Bernhard Scholz and Herbert Jordan and Pavle Subotic and Till Westmann}, title = {On fast large-scale program analysis in datalog}, year = {2016}, booktitle = {CC}}"#)),
        mock_note("tar75", "Efficiency of a Good But Not Linear Set Union Algorithm", Some("10.1145/321879.321884"), None,
            Some(r#"@article{tarjan1975, author = {Robert Endre Tarjan}, title = {Efficiency of a Good But Not Linear Set Union Algorithm}, year = {1975}, journal = {J. ACM}}"#)),
        mock_note("nel80", "Fast Decision Procedures Based on Congruence Closure", Some("10.1145/322186.322198"), None,
            Some(r#"@article{nelson1980, author = {Greg Nelson and Derek C. Oppen}, title = {Fast Decision Procedures Based on Congruence Closure}, year = {1980}, journal = {J. ACM}}"#)),
        mock_note("shi91", "Control-Flow Analysis of Higher-Order Languages", None, None,
            Some(r#"@phdthesis{shivers1991, author = {Olin G. Shivers}, title = {Control-Flow Analysis of Higher-Order Languages}, year = {1991}, school = {Carnegie Mellon University}}"#)),
        mock_note("fla93", "The essence of compiling with continuations", None, None,
            Some(r#"@inproceedings{flanagan1993, author = {Cormac Flanagan and Amr Sabry and Bruce F. Duba and Matthias Felleisen}, title = {The essence of compiling with continuations}, year = {1993}, booktitle = {PLDI}}"#)),
        mock_note("hod11", "muZ - An efficient engine for fixed points with constraints", None, None,
            Some(r#"@inproceedings{hoder2011, author = {Kryštof Hoder and Nikolaj Bjørner and Leonardo de Moura}, title = {µZ – An efficient engine for fixed points with constraints}, year = {2011}, booktitle = {CAV}}"#)),
        mock_note("mid12", "Control-flow analysis of functional programs", None, None,
            Some(r#"@article{midtgaard2012, author = {Jan Midtgaard}, title = {Control-flow analysis of functional programs}, year = {2012}, journal = {ACM Computing Surveys}}"#)),
        mock_note("mig06", "Improving flow analyses via Gamma-CFA: Abstract garbage collection and counting", None, None,
            Some(r#"@inproceedings{might2006, author = {Matthew Might and Olin Shivers}, title = {Improving flow analyses via ΓCFA: Abstract garbage collection and counting}, year = {2006}, booktitle = {ICFP}}"#)),
    ]
}

// ============================================================================
// Reference Section Detection Tests
// ============================================================================

#[test]
fn test_extract_references_from_text_souffle() {
    let text = load_fixture("jordan2016souffle.txt");
    let refs = extract_references_from_text(&text);
    // Souffle paper has 18 numbered references [1]-[18]
    assert!(refs.len() >= 15, "Expected ≥15 refs from Souffle paper, got {}", refs.len());
    assert!(refs.len() <= 22, "Expected ≤22 refs from Souffle paper, got {}", refs.len());
    // First ref should mention Abiteboul
    assert!(refs[0].contains("Abiteboul"), "First ref should mention Abiteboul, got: {}", &refs[0][..80.min(refs[0].len())]);
}

#[test]
fn test_extract_references_from_text_eqsat() {
    let text = load_fixture("willsey2021eqsat.txt");
    let refs = extract_references_from_text(&text);
    // egg paper has many references in author-year style
    assert!(refs.len() >= 20, "Expected ≥20 refs from egg paper, got {}", refs.len());
}

#[test]
fn test_extract_references_from_text_vanhorn() {
    let text = load_fixture("vanhorn2010.txt");
    let refs = extract_references_from_text(&text);
    // VanHorn2010 has [1]-[31] bracketed references
    assert!(refs.len() >= 25, "Expected ≥25 refs from VanHorn paper, got {}", refs.len());
}

#[test]
fn test_extract_references_from_text_gilray() {
    let text = load_fixture("gilray2016p4f.txt");
    let refs = extract_references_from_text(&text);
    // P4F paper has [1]-[16] bracketed references
    assert!(refs.len() >= 12, "Expected ≥12 refs from P4F paper, got {}", refs.len());
}

#[test]
fn test_extract_references_from_text_might() {
    let text = load_fixture("might2010mcfa.txt");
    let refs = extract_references_from_text(&text);
    // Might 2010 is a multi-column paper — pdftotext without -layout only captures
    // refs [1]-[3] after the "References" heading (rest end up in different column order).
    // At runtime, extract_pdf_text_best tries layout mode and pdf-extract too, which do better.
    assert!(refs.len() >= 2, "Expected ≥2 refs from Might 2010 paper, got {}", refs.len());
}

// ============================================================================
// Title Extraction Tests
// ============================================================================

#[test]
fn test_extract_title_quoted() {
    // Style with quoted title
    let reftext = r#"[5] C. Cifuentes, A. Gross, N. Keynes. "Understanding caller-sensitive method vulnerabilities: A class of access control vulnerabilities in the java platform." In SOAP 2015, ACM, 2015."#;
    let title = extract_title_from_ref(reftext);
    assert!(title.is_some(), "Should extract quoted title");
    let t = title.unwrap();
    assert!(t.contains("caller-sensitive"), "Title should contain 'caller-sensitive', got: {}", t);
}

#[test]
fn test_extract_title_post_author_year() {
    // APA-like style: Author, A. and Author, B. 2010. Title here. In Venue.
    let reftext = "David Van Horn and Matthew Might. 2010. Abstracting abstract machines. In ICFP '10, ACM, New York, NY, USA.";
    let title = extract_title_from_ref(reftext);
    assert!(title.is_some(), "Should extract post-author title");
    let t = title.unwrap();
    assert!(t.contains("Abstracting abstract machines"), "Got: {}", t);
}

#[test]
fn test_extract_title_numbered_bracket() {
    // Bracketed number style: [1] Author, F. and Author, G. Title. In Venue, year.
    let reftext = "[1] Abiteboul, S., Hull, R., Vianu, V.: Foundations of Databases. Addison-Wesley (1995)";
    let title = extract_title_from_ref(reftext);
    assert!(title.is_some(), "Should extract title from bracket-numbered ref");
    let t = title.unwrap();
    // Should get "Foundations of Databases" or similar
    assert!(t.contains("Foundations") || t.contains("Databases"),
        "Title should contain 'Foundations' or 'Databases', got: {}", t);
}

#[test]
fn test_extract_title_dot_numbered() {
    // Dot-numbered style: 1. Author, F. Title here. Venue, year.
    let reftext = "1. Abiteboul, S., Hull, R., Vianu, V.: Foundations of Databases. Addison-Wesley (1995)";
    let title = extract_title_from_ref(reftext);
    assert!(title.is_some());
}

// ============================================================================
// Author Extraction Tests
// ============================================================================

#[test]
fn test_extract_authors_comma_initial() {
    // "LastName, F. and LastName, G." style
    let reftext = "[1] Cousot, P. and Cousot, R. Systematic design of program analysis frameworks. In POPL '79, 1979.";
    let authors = extract_author_lastnames(reftext);
    assert!(authors.contains(&"cousot".to_string()), "Should find Cousot, got: {:?}", authors);
}

#[test]
fn test_extract_authors_natural_order() {
    // "FirstName LastName" style
    let reftext = "David Van Horn and Matthew Might. 2010. Abstracting abstract machines.";
    let authors = extract_author_lastnames(reftext);
    assert!(authors.iter().any(|a| a == "horn" || a == "van"), "Should find Horn or Van, got: {:?}", authors);
    assert!(authors.contains(&"might".to_string()), "Should find Might, got: {:?}", authors);
}

#[test]
fn test_extract_authors_et_al() {
    // "et al." should stop author extraction
    let reftext = "[3] Scholz, B. et al. On fast large-scale program analysis in datalog. CC 2016.";
    let authors = extract_author_lastnames(reftext);
    assert!(authors.contains(&"scholz".to_string()), "Should find Scholz, got: {:?}", authors);
    // Should not include words from the title
    assert!(!authors.contains(&"datalog".to_string()), "Should not extract 'datalog' as author");
}

#[test]
fn test_extract_authors_skip_words() {
    // Common non-name words should be filtered
    let reftext = "[1] International Conference Proceedings. Springer, 2020.";
    let authors = extract_author_lastnames(reftext);
    assert!(!authors.contains(&"international".to_string()));
    assert!(!authors.contains(&"conference".to_string()));
    assert!(!authors.contains(&"proceedings".to_string()));
    assert!(!authors.contains(&"springer".to_string()));
}

// ============================================================================
// Edit Distance Tests
// ============================================================================

#[test]
fn test_edit_distance_identical() {
    assert_eq!(edit_distance("hello", "hello"), 0);
}

#[test]
fn test_edit_distance_one_char() {
    assert_eq!(edit_distance("hello", "hallo"), 1);
    assert_eq!(edit_distance("hello", "hell"), 1);
    assert_eq!(edit_distance("hello", "helloo"), 1);
}

#[test]
fn test_edit_distance_empty() {
    assert_eq!(edit_distance("", "abc"), 3);
    assert_eq!(edit_distance("abc", ""), 3);
}

#[test]
fn test_edit_distance_swap() {
    // Transposition is 2 ops in Levenshtein (delete + insert)
    assert_eq!(edit_distance("ab", "ba"), 2);
}

// ============================================================================
// Note Pool Matching Tests
// ============================================================================

#[test]
fn test_match_by_doi() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: Some("10.1145/321879.321884".to_string()),
        arxiv_id: None,
        title: None,
        authors: vec![],
        year: None,
    };
    let m = index.match_reference(&reference);
    assert!(m.is_some(), "Should match by DOI");
    let m = m.unwrap();
    assert_eq!(m.target_key, "tar75");
    assert_eq!(m.match_type, "doi");
    assert_eq!(m.confidence, 1.0);
}

#[test]
fn test_match_by_exact_title() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: Some("Systematic design of program analysis frameworks".to_string()),
        authors: vec![],
        year: None,
    };
    let m = index.match_reference(&reference);
    assert!(m.is_some(), "Should match by exact title");
    let m = m.unwrap();
    assert_eq!(m.target_key, "cou79");
    assert_eq!(m.match_type, "title");
    assert_eq!(m.confidence, 0.90);
}

#[test]
fn test_match_by_fuzzy_title() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    // Slightly different title: "program analysis framework" vs "program analysis frameworks"
    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: Some("Systematic design of program analysis framework".to_string()),
        authors: vec![],
        year: None,
    };
    let m = index.match_reference(&reference);
    assert!(m.is_some(), "Should fuzzy-match title with 1 char difference");
    let m = m.unwrap();
    assert_eq!(m.target_key, "cou79");
    assert_eq!(m.match_type, "title_fuzzy");
    assert!(m.confidence >= 0.80, "Fuzzy match confidence should be ≥0.80, got {}", m.confidence);
}

#[test]
fn test_no_match_different_title() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    // Completely different title — should NOT match anything
    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: Some("A completely unrelated paper about quantum computing".to_string()),
        authors: vec![],
        year: None,
    };
    let m = index.match_reference(&reference);
    assert!(m.is_none(), "Should not match unrelated title");
}

#[test]
fn test_match_by_author_year() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    // Match Tarjan 1975 by author+year (only one Tarjan paper in pool)
    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: None,
        authors: vec!["tarjan".to_string()],
        year: Some(1975),
    };
    let m = index.match_reference(&reference);
    assert!(m.is_some(), "Should match Tarjan 1975 by author+year");
    let m = m.unwrap();
    assert_eq!(m.target_key, "tar75");
    assert_eq!(m.match_type, "author_year");
}

#[test]
fn test_author_year_vote_counting() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    // Multiple authors from the same paper: Scholz, Jordan, Subotic, Westmann (2016)
    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: None,
        authors: vec!["scholz".to_string(), "jordan".to_string(), "subotic".to_string()],
        year: Some(2016),
    };
    let m = index.match_reference(&reference);
    assert!(m.is_some(), "Should match by multiple author+year votes");
    let m = m.unwrap();
    assert_eq!(m.target_key, "sch16");
    assert!(m.confidence >= 0.55, "3 author matches should give confidence ≥0.55");
}

// ============================================================================
// End-to-End Pipeline Tests (fixture-based)
// ============================================================================

#[test]
fn test_e2e_souffle_paper() {
    let text = load_fixture("jordan2016souffle.txt");
    let refs = extract_references_from_text(&text);
    assert!(refs.len() >= 15);

    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let parsed: Vec<ExtractedReference> = refs
        .iter()
        .enumerate()
        .map(|(i, r)| parse_reference_text(r, i))
        .collect();

    let matches: Vec<CitationMatch> = parsed
        .iter()
        .filter_map(|r| index.match_reference(r))
        .collect();

    // Should find at least Abiteboul 1995 and Whaley 2005 from the Souffle ref list
    let matched_keys: Vec<&str> = matches.iter().map(|m| m.target_key.as_str()).collect();
    assert!(matched_keys.contains(&"abi95"), "Should match Abiteboul 1995. Matched: {:?}", matched_keys);
    assert!(matched_keys.contains(&"wha05"), "Should match Whaley 2005. Matched: {:?}", matched_keys);
    assert!(matched_keys.contains(&"sma14"), "Should match Smaragdakis 2014. Matched: {:?}", matched_keys);
    assert!(matched_keys.contains(&"sch16"), "Should match Scholz 2016. Matched: {:?}", matched_keys);
}

#[test]
fn test_e2e_vanhorn_paper() {
    let text = load_fixture("vanhorn2010.txt");
    let refs = extract_references_from_text(&text);
    assert!(refs.len() >= 25);

    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let parsed: Vec<ExtractedReference> = refs
        .iter()
        .enumerate()
        .map(|(i, r)| parse_reference_text(r, i))
        .collect();

    let matches: Vec<CitationMatch> = parsed
        .iter()
        .filter_map(|r| index.match_reference(r))
        .collect();

    let matched_keys: Vec<&str> = matches.iter().map(|m| m.target_key.as_str()).collect();
    // VanHorn cites Cousot, Might, Shivers, Flanagan
    assert!(matched_keys.contains(&"cou79"), "Should match Cousot 1979. Matched: {:?}", matched_keys);
    assert!(matched_keys.contains(&"shi91"), "Should match Shivers 1991. Matched: {:?}", matched_keys);
    assert!(matched_keys.contains(&"fla93"), "Should match Flanagan 1993. Matched: {:?}", matched_keys);
}

#[test]
fn test_e2e_no_false_positives() {
    // Parse a paper and verify no obviously wrong matches
    let text = load_fixture("jordan2016souffle.txt");
    let refs = extract_references_from_text(&text);

    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let parsed: Vec<ExtractedReference> = refs
        .iter()
        .enumerate()
        .map(|(i, r)| parse_reference_text(r, i))
        .collect();

    let matches: Vec<CitationMatch> = parsed
        .iter()
        .filter_map(|r| index.match_reference(r))
        .collect();

    // The Souffle paper should NOT match: Tarjan, Nelson/Oppen, Tate (eqsat),
    // VanHorn, Flanagan (not cited)
    let matched_keys: Vec<&str> = matches.iter().map(|m| m.target_key.as_str()).collect();
    assert!(!matched_keys.contains(&"tar75"), "Should NOT match Tarjan (not cited). Matched: {:?}", matched_keys);
    assert!(!matched_keys.contains(&"nel80"), "Should NOT match Nelson (not cited). Matched: {:?}", matched_keys);
    assert!(!matched_keys.contains(&"tat09"), "Should NOT match Tate (not cited). Matched: {:?}", matched_keys);
    assert!(!matched_keys.contains(&"vh10"), "Should NOT match VanHorn (not cited). Matched: {:?}", matched_keys);
}

// ============================================================================
// Normalize Title Tests
// ============================================================================

#[test]
fn test_normalize_title_basic() {
    assert_eq!(normalize_title("Hello World"), "hello world");
    assert_eq!(normalize_title("Hello,  World!"), "hello world");
    assert_eq!(normalize_title("  Lots   of   Spaces  "), "lots of spaces");
}

#[test]
fn test_normalize_title_special_chars() {
    // Hyphens, colons, etc. become spaces then collapse
    assert_eq!(
        normalize_title("k-CFA: Fast and Precise"),
        "k cfa fast and precise"
    );
}

// ============================================================================
// Reference Splitting Tests
// ============================================================================

#[test]
fn test_split_by_pattern_brackets() {
    let text = "[1] First ref.\n[2] Second ref.\n[3] Third ref.";
    let pat = Regex::new(r"(?m)^\s*\[(\d+)\]").unwrap();
    let entries = split_by_pattern(text, &pat);
    assert_eq!(entries.len(), 3);
    assert!(entries[0].contains("First"));
    assert!(entries[2].contains("Third"));
}

#[test]
fn test_split_blank_lines_merges_fragments() {
    let text = "This is a long reference entry that spans multiple lines and is substantial enough.\n\nShort frag\n\nAnother full reference entry with enough text to be meaningful on its own.";
    let entries = split_by_blank_lines(text);
    // "Short frag" (10 chars) should be filtered out (< 40)
    // We should get 2 entries, and the short frag is too short to even be kept
    assert_eq!(entries.len(), 2);
}

// ============================================================================
// Confidence Calibration Tests
// ============================================================================

#[test]
fn test_confidence_doi_is_max() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: Some("10.1145/321879.321884".to_string()),
        arxiv_id: None,
        title: None,
        authors: vec![],
        year: None,
    };
    let m = index.match_reference(&reference).unwrap();
    assert_eq!(m.confidence, 1.0, "DOI match should have confidence 1.0");
}

#[test]
fn test_confidence_title_exact_is_high() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    let reference = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: Some("Fast Decision Procedures Based on Congruence Closure".to_string()),
        authors: vec![],
        year: None,
    };
    let m = index.match_reference(&reference).unwrap();
    assert_eq!(m.confidence, 0.90, "Exact title match should have confidence 0.90");
}

#[test]
fn test_confidence_author_year_scales_with_votes() {
    let pool = build_test_pool();
    let index = NotePoolIndex::build(&pool);

    // 1 author match
    let ref1 = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: None,
        authors: vec!["tarjan".to_string()],
        year: Some(1975),
    };
    let m1 = index.match_reference(&ref1).unwrap();
    assert_eq!(m1.confidence, 0.40, "1-author match should give 0.40");

    // 3 author matches
    let ref3 = ExtractedReference {
        raw_text: "test".to_string(),
        index: 0,
        doi: None,
        arxiv_id: None,
        title: None,
        authors: vec!["scholz".to_string(), "jordan".to_string(), "subotic".to_string()],
        year: Some(2016),
    };
    let m3 = index.match_reference(&ref3).unwrap();
    assert!(m3.confidence >= 0.55, "3-author match should give ≥0.55, got {}", m3.confidence);
}

// ============================================================================
// Parse Reference Text Tests
// ============================================================================

#[test]
fn test_parse_reference_extracts_doi() {
    let text = "Tate et al. Equality Saturation. POPL 2009. https://doi.org/10.1145/1480881.1480915";
    let parsed = parse_reference_text(text, 0);
    assert!(parsed.doi.is_some(), "Should extract DOI");
    assert!(parsed.doi.unwrap().contains("1480881"));
}

#[test]
fn test_parse_reference_extracts_year() {
    let text = "[1] Cousot, P. and Cousot, R. 1979. Systematic design of program analysis frameworks.";
    let parsed = parse_reference_text(text, 0);
    assert_eq!(parsed.year, Some(1979));
}

#[test]
fn test_parse_reference_extracts_authors() {
    let text = "[1] Cousot, P. and Cousot, R. 1979. Systematic design of program analysis frameworks.";
    let parsed = parse_reference_text(text, 0);
    assert!(parsed.authors.contains(&"cousot".to_string()));
}
