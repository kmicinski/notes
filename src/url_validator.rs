//! URL validation module for SSRF prevention.
//!
//! This module provides URL validation to prevent Server-Side Request Forgery (SSRF)
//! attacks by enforcing:
//! - HTTPS-only connections
//! - Domain allowlist for trusted sources
//! - Internal IP address blocking (private ranges, loopback, link-local)
//! - DNS rebinding protection

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use url::Url;

/// Allowed domains for URL fetching (academic sources and publishers)
const ALLOWED_DOMAINS: &[&str] = &[
    // Academic repositories
    "arxiv.org",
    "export.arxiv.org",
    // DOI and CrossRef
    "doi.org",
    "dx.doi.org",
    "api.crossref.org",
    "crossref.org",
    // Major publishers
    "dl.acm.org",
    "ieeexplore.ieee.org",
    "ieee.org",
    "link.springer.com",
    "springer.com",
    "sciencedirect.com",
    "elsevier.com",
    "nature.com",
    "science.org",
    "pnas.org",
    "wiley.com",
    "onlinelibrary.wiley.com",
    "tandfonline.com",
    "journals.plos.org",
    "plos.org",
    "oup.com",
    "academic.oup.com",
    "cambridge.org",
    "jstor.org",
    "ssrn.com",
    "researchgate.net",
    "semanticscholar.org",
    "api.semanticscholar.org",
    "openreview.net",
    "aclweb.org",
    "aclanthology.org",
    "neurips.cc",
    "proceedings.neurips.cc",
    "mlr.press",
    "proceedings.mlr.press",
    "aaai.org",
    "ijcai.org",
    "usenix.org",
];

/// Result of URL validation
#[derive(Debug, Clone)]
pub enum UrlValidationError {
    /// URL is malformed or cannot be parsed
    InvalidUrl(String),
    /// URL uses non-HTTPS scheme
    NotHttps,
    /// Domain is not in the allowlist
    DomainNotAllowed(String),
    /// Resolved IP is a private/internal address
    InternalIpAddress(String),
    /// DNS resolution failed
    DnsResolutionFailed(String),
}

impl std::fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UrlValidationError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            UrlValidationError::NotHttps => write!(f, "Only HTTPS URLs are allowed"),
            UrlValidationError::DomainNotAllowed(domain) => {
                write!(f, "Domain not in allowlist: {}", domain)
            }
            UrlValidationError::InternalIpAddress(ip) => {
                write!(f, "Internal IP addresses are not allowed: {}", ip)
            }
            UrlValidationError::DnsResolutionFailed(msg) => {
                write!(f, "DNS resolution failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for UrlValidationError {}

/// Check if an IPv4 address is internal/private
fn is_internal_ipv4(ip: &Ipv4Addr) -> bool {
    // Loopback (127.0.0.0/8)
    ip.is_loopback()
    // Private networks
    || ip.is_private()
    // Link-local (169.254.0.0/16)
    || ip.is_link_local()
    // Broadcast
    || ip.is_broadcast()
    // Documentation (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24)
    || (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 2)
    || (ip.octets()[0] == 198 && ip.octets()[1] == 51 && ip.octets()[2] == 100)
    || (ip.octets()[0] == 203 && ip.octets()[1] == 0 && ip.octets()[2] == 113)
    // Shared address space (100.64.0.0/10)
    || (ip.octets()[0] == 100 && (ip.octets()[1] & 0xC0) == 64)
    // IETF protocol assignments (192.0.0.0/24)
    || (ip.octets()[0] == 192 && ip.octets()[1] == 0 && ip.octets()[2] == 0)
    // Benchmarking (198.18.0.0/15)
    || (ip.octets()[0] == 198 && (ip.octets()[1] == 18 || ip.octets()[1] == 19))
    // Unspecified
    || ip.is_unspecified()
}

/// Check if an IPv6 address is internal/private
fn is_internal_ipv6(ip: &Ipv6Addr) -> bool {
    // Loopback (::1)
    ip.is_loopback()
    // Unspecified (::)
    || ip.is_unspecified()
    // IPv4-mapped addresses - check the embedded IPv4
    || ip.to_ipv4_mapped().map(|v4| is_internal_ipv4(&v4)).unwrap_or(false)
    // Unique local addresses (fc00::/7)
    || (ip.segments()[0] & 0xFE00) == 0xFC00
    // Link-local (fe80::/10)
    || (ip.segments()[0] & 0xFFC0) == 0xFE80
    // Documentation (2001:db8::/32)
    || (ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0DB8)
}

/// Check if an IP address is internal/private
fn is_internal_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_internal_ipv4(v4),
        IpAddr::V6(v6) => is_internal_ipv6(v6),
    }
}

/// Check if a domain is in the allowlist
fn is_domain_allowed(host: &str) -> bool {
    let host_lower = host.to_lowercase();

    for allowed in ALLOWED_DOMAINS {
        // Exact match
        if host_lower == *allowed {
            return true;
        }
        // Subdomain match (e.g., "www.arxiv.org" matches "arxiv.org")
        if host_lower.ends_with(&format!(".{}", allowed)) {
            return true;
        }
    }

    false
}

/// Validate a URL for safe fetching (SSRF protection)
///
/// This function performs the following checks:
/// 1. Parses the URL and validates it's well-formed
/// 2. Ensures the scheme is HTTPS
/// 3. Checks the domain against the allowlist
/// 4. Resolves the domain and checks the IP isn't internal
///
/// # Arguments
/// * `url_str` - The URL string to validate
///
/// # Returns
/// * `Ok(Url)` - The parsed and validated URL
/// * `Err(UrlValidationError)` - If validation fails
pub fn validate_url(url_str: &str) -> Result<Url, UrlValidationError> {
    // Parse the URL
    let url = Url::parse(url_str).map_err(|e| UrlValidationError::InvalidUrl(e.to_string()))?;

    // Check scheme is HTTPS
    if url.scheme() != "https" {
        return Err(UrlValidationError::NotHttps);
    }

    // Get the host
    let host = url
        .host_str()
        .ok_or_else(|| UrlValidationError::InvalidUrl("No host in URL".to_string()))?;

    // Check domain allowlist
    if !is_domain_allowed(host) {
        return Err(UrlValidationError::DomainNotAllowed(host.to_string()));
    }

    // DNS resolution and IP check (DNS rebinding protection)
    let port = url.port().unwrap_or(443);
    let socket_addr = format!("{}:{}", host, port);

    match socket_addr.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                if is_internal_ip(&addr.ip()) {
                    return Err(UrlValidationError::InternalIpAddress(
                        addr.ip().to_string(),
                    ));
                }
            }
        }
        Err(e) => {
            return Err(UrlValidationError::DnsResolutionFailed(e.to_string()));
        }
    }

    Ok(url)
}

/// Validate a URL, allowing HTTP for specific trusted API endpoints
///
/// Some APIs (like arXiv export) may not support HTTPS consistently.
/// This is a more permissive validator that should only be used for
/// known-safe API calls.
pub fn validate_api_url(url_str: &str) -> Result<Url, UrlValidationError> {
    let url = Url::parse(url_str).map_err(|e| UrlValidationError::InvalidUrl(e.to_string()))?;

    // Allow both HTTP and HTTPS for API endpoints
    if url.scheme() != "https" && url.scheme() != "http" {
        return Err(UrlValidationError::NotHttps);
    }

    let host = url
        .host_str()
        .ok_or_else(|| UrlValidationError::InvalidUrl("No host in URL".to_string()))?;

    if !is_domain_allowed(host) {
        return Err(UrlValidationError::DomainNotAllowed(host.to_string()));
    }

    let port = url.port().unwrap_or(if url.scheme() == "https" { 443 } else { 80 });
    let socket_addr = format!("{}:{}", host, port);

    match socket_addr.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                if is_internal_ip(&addr.ip()) {
                    return Err(UrlValidationError::InternalIpAddress(
                        addr.ip().to_string(),
                    ));
                }
            }
        }
        Err(e) => {
            return Err(UrlValidationError::DnsResolutionFailed(e.to_string()));
        }
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_domains() {
        assert!(is_domain_allowed("arxiv.org"));
        assert!(is_domain_allowed("export.arxiv.org"));
        assert!(is_domain_allowed("www.arxiv.org"));
        assert!(is_domain_allowed("api.crossref.org"));
        assert!(!is_domain_allowed("evil.com"));
        assert!(!is_domain_allowed("arxiv.org.evil.com"));
    }

    #[test]
    fn test_internal_ips() {
        // IPv4
        assert!(is_internal_ipv4(&Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_internal_ipv4(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_internal_ipv4(&Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_internal_ipv4(&Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_internal_ipv4(&Ipv4Addr::new(169, 254, 1, 1)));
        assert!(!is_internal_ipv4(&Ipv4Addr::new(8, 8, 8, 8)));

        // IPv6
        assert!(is_internal_ipv6(&Ipv6Addr::LOCALHOST));
        assert!(is_internal_ipv6(&Ipv6Addr::UNSPECIFIED));
    }

    #[test]
    fn test_validate_url_rejects_http() {
        let result = validate_url("http://arxiv.org/abs/1234.5678");
        assert!(matches!(result, Err(UrlValidationError::NotHttps)));
    }

    #[test]
    fn test_validate_url_rejects_unknown_domain() {
        let result = validate_url("https://evil.com/malicious");
        assert!(matches!(
            result,
            Err(UrlValidationError::DomainNotAllowed(_))
        ));
    }
}
