# Deployment Guide: Notes App on Mac Mini via DuckDNS

This document covers a complete security audit, remediation plan, and deployment
instructions for exposing the notes application to the internet on a Mac Mini
using DuckDNS dynamic DNS.

---

## Table of Contents

1. [Security Audit Findings](#1-security-audit-findings)
2. [Required Code Fixes Before Deployment](#2-required-code-fixes-before-deployment)
3. [Infrastructure Architecture](#3-infrastructure-architecture)
4. [Mac Mini Setup](#4-mac-mini-setup)
5. [DuckDNS Configuration](#5-duckdns-configuration)
6. [TLS with Caddy (Reverse Proxy)](#6-tls-with-caddy-reverse-proxy)
7. [Docker Deployment](#7-docker-deployment)
8. [Directory Isolation (PARAMOUNT)](#8-directory-isolation-paramount)
9. [Firewall and Network Hardening](#9-firewall-and-network-hardening)
10. [Monitoring and Alerting](#10-monitoring-and-alerting)
11. [Penetration Testing](#11-penetration-testing)
12. [Backup Strategy](#12-backup-strategy)
13. [Maintenance Runbook](#13-maintenance-runbook)

---

## 1. Security Audit Findings

### CRITICAL Severity

#### 1.1 No Session Cookie `Secure` Flag

**Location:** `src/handlers.rs:633-634`

The session cookie is set with `HttpOnly` and `SameSite=Strict` (good), but is
**missing the `Secure` flag**. Over the internet with HTTPS, this is mandatory
— without it, the cookie can be sent over plain HTTP if an attacker downgrades
the connection.

```rust
// Current (INSECURE for production):
"{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}"

// Required:
"{}={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}"
```

#### 1.2 No Rate Limiting on Login

**Location:** `src/handlers.rs:604`

The `/login` POST endpoint has zero rate limiting. An attacker can brute-force
passwords at network speed. There is no lockout, no delay, no logging of failed
attempts.

#### 1.3 Timing-Vulnerable Password Comparison

**Location:** `src/handlers.rs:611`

```rust
if form.password != password {
```

Standard `!=` on strings short-circuits on the first differing byte, leaking
password length and character information through timing side-channels.

#### 1.4 Command Injection via Claude CLI

**Location:** `src/smart_add.rs:737-746`

User-supplied URLs are interpolated directly into a string that is passed as a
`-p` argument to the `claude` CLI. While `Command::new` with `.args()` does
prevent shell metacharacter interpretation, the URL still becomes part of a
prompt that controls an LLM — this is a prompt injection vector that could
cause Claude to output arbitrary content that gets parsed as JSON and inserted
into notes.

#### 1.5 No Security Headers

The application sets **zero** security headers. No `Content-Security-Policy`,
no `X-Frame-Options`, no `X-Content-Type-Options`, no
`Strict-Transport-Security`. This leaves the app vulnerable to clickjacking,
MIME-sniffing attacks, and XSS escalation.

### HIGH Severity

#### 1.6 Markdown Renders Raw HTML (XSS)

**Location:** `src/notes.rs:408-413`

`pulldown_cmark` renders raw HTML from markdown content with no sanitization.
An attacker who can write a note (i.e., anyone logged in, which is the user)
could embed `<script>` tags. In a single-user app this is self-XSS, but if a
note's URL is shared, cached by a proxy, or if the app gains multi-user
features, this becomes a stored XSS vulnerability.

```markdown
<img src=x onerror="fetch('/api/note/abc123',{method:'DELETE'})">
```

#### 1.7 Incomplete Path Traversal Protection

**Location:** `src/handlers.rs:797`

The check `filename.contains("..")` is insufficient. It does not prevent:
- Absolute paths: `filename = "/etc/passwd"` — `PathBuf::join` with an
  absolute path *replaces* the base, so `notes_dir.join("/etc/passwd")` yields
  `/etc/passwd`.
- Encoded traversals or other creative bypasses.

The correct fix is post-join canonicalization:

```rust
let file_path = state.notes_dir.join(filename);
let canonical = file_path.canonicalize()?;
if !canonical.starts_with(state.notes_dir.canonicalize()?) {
    return error_response("Invalid path");
}
```

#### 1.8 Custom Base64 Implementation

**Location:** `src/auth.rs:123-169`

Hand-rolled base64 encoding/decoding instead of using the `base64` crate. This
is a correctness risk in a security-critical path (session token
creation/verification).

#### 1.9 `.git` Directory Mounted Read-Write

**Location:** `docker-compose.yml:17`

```yaml
- ./.git:/app/.git:rw
```

If an attacker achieves write access inside the container, they could plant
malicious git hooks (`.git/hooks/post-commit`) that execute arbitrary code on
the next git operation. Mount as read-only or use `git2` library.

### MEDIUM Severity

#### 1.10 No CSRF Protection

State-changing endpoints (`POST /api/note/{key}`, `DELETE /api/note/{key}`,
`POST /api/pdf/upload`) have no CSRF tokens. The `SameSite=Strict` cookie
mitigates most CSRF scenarios, but defense-in-depth dictates adding tokens.

#### 1.11 PDF Filename Sanitization Weak

**Location:** `src/handlers.rs:1217`

Only strips `/` and `\`. Does not strip null bytes, shell metacharacters, or
validate length. Could allow unusual filenames that cause problems on some
filesystems.

#### 1.12 Git Command Errors Silently Ignored

**Location:** `src/handlers.rs:442-451`

All `Command::new("git")` calls discard errors with `let _ = ...`. Failed git
operations are invisible to the user and to any monitoring system.

#### 1.13 Session Signature Comparison Not Constant-Time

**Location:** `src/auth.rs:92`

```rust
if parts[1] != expected_sig {
```

The HMAC signature comparison uses standard string equality, which is
vulnerable to timing attacks. Use `hmac::Mac::verify_slice` or the `subtle`
crate for constant-time comparison.

### POSITIVE Findings

- HMAC-SHA256 session tokens with nonce and expiration — good design.
- SSRF protection in `url_validator.rs` — comprehensive domain allowlist,
  internal IP blocking, DNS rebinding protection.
- Docker hardening is solid: non-root user, read-only filesystem, dropped
  capabilities, seccomp profile, resource limits.
- The app binds to `127.0.0.1` only — never directly exposed.
- Cookie uses `HttpOnly` and `SameSite=Strict`.

---

## 2. Required Code Fixes Before Deployment

These must be completed before the app goes on the internet.

### Fix 1: Add `Secure` flag to session cookie

In `src/handlers.rs`, change the cookie format string to include `Secure`:

```rust
let cookie = format!(
    "{}={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
    SESSION_COOKIE, session_token, SESSION_TTL_HOURS * 3600
);
```

Also add `Secure` to the logout cookie.

### Fix 2: Constant-time password comparison

Add `subtle` to `Cargo.toml`:

```toml
subtle = "2.5"
```

In `src/handlers.rs`:

```rust
use subtle::ConstantTimeEq;

let password = std::env::var("NOTES_PASSWORD").unwrap_or_default();
let input_bytes = form.password.as_bytes();
let expected_bytes = password.as_bytes();

// Constant-time comparison (pad to equal length first)
let matches = if input_bytes.len() == expected_bytes.len() {
    input_bytes.ct_eq(expected_bytes).into()
} else {
    false
};

if !matches {
    // return error
}
```

### Fix 3: Constant-time HMAC verification

In `src/auth.rs:92`, replace:

```rust
if parts[1] != expected_sig {
```

With:

```rust
use subtle::ConstantTimeEq;
if parts[1].as_bytes().ct_eq(expected_sig.as_bytes()).unwrap_u8() != 1 {
```

### Fix 4: Path traversal canonicalization

In every handler that constructs a file path from user input, add:

```rust
let file_path = state.notes_dir.join(filename);
let notes_dir_canonical = std::fs::canonicalize(&state.notes_dir)
    .map_err(|_| /* error response */)?;

// For new files that don't exist yet, canonicalize the parent
let parent = file_path.parent().unwrap_or(&file_path);
let parent_canonical = std::fs::canonicalize(parent)
    .map_err(|_| /* error response */)?;

if !parent_canonical.starts_with(&notes_dir_canonical) {
    return /* error: path traversal detected */;
}
```

### Fix 5: Replace custom base64 with `base64` crate

Add to `Cargo.toml`:

```toml
base64 = "0.22"
```

Replace the custom implementations in `src/auth.rs` with:

```rust
use base64::{Engine, engine::general_purpose::STANDARD};

pub fn base64_encode(s: &str) -> String {
    STANDARD.encode(s.as_bytes())
}

pub fn base64_decode(s: &str) -> Option<String> {
    let bytes = STANDARD.decode(s).ok()?;
    String::from_utf8(bytes).ok()
}
```

### Fix 6: Sanitize markdown HTML output

Add to `Cargo.toml`:

```toml
ammonia = "4"
```

In `src/notes.rs`:

```rust
pub fn render_markdown(content: &str) -> String {
    let parser = Parser::new(content);
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    ammonia::clean(&html_output)
}
```

### Fix 7: Mount `.git` as read-only

In `docker-compose.yml`:

```yaml
- ./.git:/app/.git:ro
```

This means git commits from within the container will fail. To preserve git
commit functionality, either:
- (a) Use a sidecar container or host-level script that watches for changes
  and commits them, or
- (b) Mount a dedicated git working directory and use `git2` in Rust, or
- (c) Accept the tradeoff: keep `:rw` but add file integrity monitoring on
  `.git/hooks/`.

For a personal notes app, option (a) is recommended — a simple cron job on the
host:

```bash
cd /path/to/notes && git add -A content/ pdfs/ && \
  git commit -m "auto-save: $(date)" 2>/dev/null || true
```

---

## 3. Infrastructure Architecture

```
Internet
   │
   ▼
[Router / NAT]  ← port 443 forwarded to Mac Mini
   │
   ▼
[macOS Firewall (pf)]  ← allow 443 only
   │
   ▼
[Caddy Reverse Proxy]  ← TLS termination, HTTPS, security headers
   │  listens on :443
   │  proxies to 127.0.0.1:3000
   ▼
[Docker Container]  ← notes app on 127.0.0.1:3000
   │  read-only filesystem
   │  non-root user
   │  seccomp + dropped caps
   │
   ├── /app/content (rw, bind mount, notes markdown)
   ├── /app/pdfs (rw, bind mount, pdf uploads)
   ├── /app/.git (ro, bind mount, version history)
   └── /app/.notes_db (tmpfs, session data)
```

**Why Caddy?** Caddy handles automatic TLS certificate provisioning via
Let's Encrypt (including the DNS challenge needed for DuckDNS), adds security
headers, handles HTTP-to-HTTPS redirect, and provides access logging — all
with minimal configuration. It's the simplest production-grade reverse proxy
for this use case.

---

## 4. Mac Mini Setup

### 4.1 macOS Hardening

```bash
# Enable the firewall
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --setglobalstate on

# Enable stealth mode (don't respond to pings)
sudo /usr/libexec/ApplicationFirewall/socketfilterfw --setstealthmode on

# Enable automatic updates
sudo softwareupdate --schedule on

# Disable unnecessary sharing services
sudo launchctl unload -w /System/Library/LaunchDaemons/com.apple.screensharing.plist 2>/dev/null
sudo launchctl unload -w /System/Library/LaunchDaemons/com.apple.AppleFileServer.plist 2>/dev/null
```

### 4.2 Create a Dedicated User

Run the app under a dedicated unprivileged user, not your main account:

```bash
sudo dscl . -create /Users/notesapp
sudo dscl . -create /Users/notesapp UserShell /bin/bash
sudo dscl . -create /Users/notesapp UniqueID 501
sudo dscl . -create /Users/notesapp PrimaryGroupID 20
sudo dscl . -create /Users/notesapp NFSHomeDirectory /Users/notesapp
sudo mkdir -p /Users/notesapp
sudo chown notesapp:staff /Users/notesapp
```

### 4.3 Install Docker Desktop for Mac

Download from https://www.docker.com/products/docker-desktop/ and install.
Enable "Start Docker Desktop when you log in" in Settings > General.

### 4.4 Install Caddy

```bash
brew install caddy
```

### 4.5 Clone the Repository

```bash
sudo -u notesapp git clone <your-repo-url> /Users/notesapp/notes
cd /Users/notesapp/notes
```

---

## 5. DuckDNS Configuration

### 5.1 Register a Subdomain

1. Go to https://www.duckdns.org and sign in.
2. Create a subdomain (e.g., `yournotes.duckdns.org`).
3. Note your **token** from the DuckDNS dashboard.

### 5.2 Automatic IP Update

Create a launchd plist to update your IP every 5 minutes:

```bash
mkdir -p /Users/notesapp/duckdns
```

Create `/Users/notesapp/duckdns/duck.sh`:

```bash
#!/bin/bash
DOMAIN="yournotes"
TOKEN="your-duckdns-token-here"
echo url="https://www.duckdns.org/update?domains=${DOMAIN}&token=${TOKEN}&ip=" \
  | curl -k -o /Users/notesapp/duckdns/duck.log -K -
```

```bash
chmod 700 /Users/notesapp/duckdns/duck.sh
```

Create `/Library/LaunchDaemons/com.duckdns.update.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.duckdns.update</string>
    <key>ProgramArguments</key>
    <array>
        <string>/Users/notesapp/duckdns/duck.sh</string>
    </array>
    <key>StartInterval</key>
    <integer>300</integer>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/Users/notesapp/duckdns/duck-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/notesapp/duckdns/duck-stderr.log</string>
    <key>UserName</key>
    <string>notesapp</string>
</dict>
</plist>
```

```bash
sudo launchctl load /Library/LaunchDaemons/com.duckdns.update.plist
```

---

## 6. TLS with Caddy (Reverse Proxy)

### 6.1 Caddyfile

Create `/Users/notesapp/Caddyfile`:

```
yournotes.duckdns.org {
    # TLS via Let's Encrypt with DuckDNS DNS challenge
    tls {
        dns duckdns {env.DUCKDNS_TOKEN}
    }

    # Security headers
    header {
        Strict-Transport-Security "max-age=63072000; includeSubDomains; preload"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
        Content-Security-Policy "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-ancestors 'none'"
        X-XSS-Protection "0"
        Permissions-Policy "camera=(), microphone=(), geolocation=()"
        -Server
    }

    # Rate limiting on login endpoint
    @login {
        path /login
        method POST
    }
    rate_limit @login {
        zone login_zone {
            key {remote_host}
            events 5
            window 1m
        }
    }

    # Proxy to the notes app
    reverse_proxy 127.0.0.1:3000 {
        header_up X-Real-IP {remote_host}
        header_up X-Forwarded-For {remote_host}
        header_up X-Forwarded-Proto {scheme}
    }

    # Access logging
    log {
        output file /Users/notesapp/logs/caddy-access.log {
            roll_size 10mb
            roll_keep 5
        }
        format json
    }
}

# Redirect HTTP to HTTPS
http://yournotes.duckdns.org {
    redir https://yournotes.duckdns.org{uri} permanent
}
```

### 6.2 Install the DuckDNS DNS Plugin

Caddy needs the DuckDNS plugin for DNS-01 challenge (required since the Mac
Mini is behind NAT and may not have port 80 available for HTTP-01):

```bash
# Install xcaddy to build Caddy with plugins
brew install xcaddy

# Build Caddy with DuckDNS support
xcaddy build --with github.com/caddy-dns/duckdns

# Replace the system Caddy binary
sudo mv caddy $(brew --prefix)/bin/caddy
```

### 6.3 Run Caddy as a Service

Create `/Library/LaunchDaemons/com.caddyserver.caddy.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.caddyserver.caddy</string>
    <key>ProgramArguments</key>
    <array>
        <string>/opt/homebrew/bin/caddy</string>
        <string>run</string>
        <string>--config</string>
        <string>/Users/notesapp/Caddyfile</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>DUCKDNS_TOKEN</key>
        <string>your-duckdns-token-here</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/Users/notesapp/logs/caddy-stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/Users/notesapp/logs/caddy-stderr.log</string>
</dict>
</plist>
```

```bash
sudo mkdir -p /Users/notesapp/logs
sudo chown notesapp:staff /Users/notesapp/logs
sudo launchctl load /Library/LaunchDaemons/com.caddyserver.caddy.plist
```

---

## 7. Docker Deployment

### 7.1 Password Management

**Never store the password in a file checked into git.** Use one of:

**Option A: macOS Keychain (recommended)**

```bash
# Store password in Keychain
security add-generic-password -a notesapp -s notes-password -w

# Retrieve it at container start
export NOTES_PASSWORD=$(security find-generic-password -a notesapp -s notes-password -w)
```

**Option B: Docker secrets with a `.env` file**

```bash
# Create .env file with restricted permissions
echo "NOTES_PASSWORD=$(openssl rand -base64 32)" > /Users/notesapp/notes/.env
chmod 600 /Users/notesapp/notes/.env
chown notesapp:staff /Users/notesapp/notes/.env
```

Add `.env` to `.gitignore` and `.dockerignore`.

**Password requirements:** Use a long (20+ character), random password. This
is a single-user app — you only need to type it once per 24 hours. Generate
one with:

```bash
openssl rand -base64 32
```

### 7.2 Production docker-compose.yml

Create `/Users/notesapp/notes/docker-compose.prod.yml`:

```yaml
version: '3.8'

services:
  notes:
    build: .
    container_name: notes-app
    ports:
      - "127.0.0.1:3000:3000"
    env_file:
      - .env
    volumes:
      # Notes content (read-write, restricted to this directory)
      - ./content:/app/content:rw
      # PDF files (read-write, restricted to this directory)
      - ./pdfs:/app/pdfs:rw
      # Git repository (READ-ONLY — see Section 2, Fix 7)
      - ./.git:/app/.git:ro
    tmpfs:
      - /tmp:size=64M,mode=1777
      - /app/.notes_db:size=32M,mode=0700,uid=1000,gid=1000

    # Security hardening
    security_opt:
      - no-new-privileges:true
      - seccomp:seccomp-profile.json
    cap_drop:
      - ALL
    read_only: true

    # Resource limits
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 512M
        reservations:
          cpus: '0.25'
          memory: 128M

    # Restart policy
    restart: unless-stopped

    # Logging
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "5"

    # Health check
    healthcheck:
      test: ["CMD", "curl", "-f", "http://127.0.0.1:3000/"]
      interval: 30s
      timeout: 3s
      start_period: 10s
      retries: 3

networks:
  default:
    driver: bridge
```

### 7.3 Build and Run

```bash
cd /Users/notesapp/notes
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d
```

### 7.4 Verify

```bash
# Check container is running
docker ps

# Check health
docker inspect --format='{{.State.Health.Status}}' notes-app

# Test locally
curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:3000/
# Should return 200

# Test HTTPS (after Caddy is running)
curl -s -o /dev/null -w "%{http_code}" https://yournotes.duckdns.org/
# Should return 200
```

---

## 8. Directory Isolation (PARAMOUNT)

This section addresses the absolute requirement that the application can
**never** modify files outside its designated directories.

### 8.1 Docker Bind Mount Isolation

The Docker container can only see files that are explicitly mounted. With the
`docker-compose.prod.yml` above, the container sees exactly:

| Container Path    | Host Path                        | Mode | Purpose           |
|-------------------|----------------------------------|------|-------------------|
| `/app/content/`   | `/Users/notesapp/notes/content/` | rw   | Markdown notes    |
| `/app/pdfs/`      | `/Users/notesapp/notes/pdfs/`    | rw   | PDF uploads       |
| `/app/.git/`      | `/Users/notesapp/notes/.git/`    | ro   | Version history   |
| `/app/.notes_db/` | (tmpfs, memory only)             | rw   | Session database  |
| `/tmp/`           | (tmpfs, memory only)             | rw   | Temp files        |

The rest of the filesystem is **read-only** (`read_only: true`). The container
has no access to the host filesystem beyond these mounts.

### 8.2 Code-Level Path Containment

Even within the container, the application code must enforce that all file
operations stay within `/app/content/` and `/app/pdfs/`. Apply Fix 4 from
Section 2 to every handler that touches the filesystem. The check must be:

```rust
fn validate_path_within(base: &Path, requested: &Path) -> Result<PathBuf, ()> {
    let canonical_base = std::fs::canonicalize(base).map_err(|_| ())?;

    // For existing files:
    if requested.exists() {
        let canonical = std::fs::canonicalize(requested).map_err(|_| ())?;
        if canonical.starts_with(&canonical_base) {
            return Ok(canonical);
        }
        return Err(());
    }

    // For new files: canonicalize the parent
    let parent = requested.parent().ok_or(())?;
    let canonical_parent = std::fs::canonicalize(parent).map_err(|_| ())?;
    if canonical_parent.starts_with(&canonical_base) {
        return Ok(requested.to_path_buf());
    }
    Err(())
}
```

### 8.3 Seccomp Profile

The seccomp profile (`seccomp-profile.json`) uses a default-deny policy
(`SCMP_ACT_ERRNO`) with an explicit allowlist. This is correct. However, note
that `execve` is allowed because the app spawns `git` and `claude` processes.

If you remove the `claude` CLI fallback and mount `.git` as read-only (making
in-container git operations pointless), you could remove `execve`, `fork`, and
`vfork` from the seccomp allowlist — this would prevent any command execution
from within the container even if an attacker achieves code execution.

### 8.4 Drop `execve` (Strongest Isolation)

If you move git commit operations to a host-side cron job (recommended), you
can remove command execution entirely. Remove from `seccomp-profile.json`:

```
"execve", "fork", "vfork"
```

This makes the container incapable of spawning any child process, even if the
application is fully compromised. This is the strongest possible isolation for
a web application.

**Tradeoff:** The Claude CLI fallback in smart_add will stop working. This is
acceptable — the app already has arXiv API, CrossRef API, and web scraping
fallbacks for metadata extraction.

### 8.5 macOS File Permissions

On the host, restrict the notes directory:

```bash
# Only notesapp user can access the notes directory
chmod 700 /Users/notesapp/notes
chown -R notesapp:staff /Users/notesapp/notes

# Content and PDFs directories: owned by notesapp,
# Docker maps uid 1000 (notes user in container) to host uid
chmod 755 /Users/notesapp/notes/content
chmod 755 /Users/notesapp/notes/pdfs
```

### 8.6 Docker User Namespace Remapping (Advanced)

For maximum isolation, enable Docker user namespace remapping so that uid 1000
inside the container maps to an unprivileged uid on the host:

In Docker Desktop settings, enable "Use containerd for pulling and storing
images" and configure user namespace remapping in `daemon.json`:

```json
{
  "userns-remap": "notesapp"
}
```

This ensures that even if an attacker escapes the container, they have no
privileges on the host.

---

## 9. Firewall and Network Hardening

### 9.1 Router Configuration

On your router:
- Forward **only port 443** (HTTPS) to the Mac Mini's local IP.
- Do **not** forward port 80 (Caddy handles HTTP→HTTPS redirect internally,
  and the DNS-01 challenge doesn't need port 80).
- Do **not** forward port 3000 or any other port.
- Disable UPnP on the router.

### 9.2 macOS pf Firewall

Create `/etc/pf.anchors/notes`:

```
# Block everything except SSH (local only) and HTTPS
block in all
pass in on lo0 all
pass out all keep state

# Allow HTTPS from internet
pass in proto tcp from any to any port 443 flags S/SA keep state

# Allow SSH from local network only
pass in proto tcp from 192.168.0.0/16 to any port 22 flags S/SA keep state

# Rate limit HTTPS connections (max 100/sec from single IP)
pass in proto tcp from any to any port 443 flags S/SA keep state \
    (max-src-conn 100, max-src-conn-rate 30/10)
```

Load it:

```bash
sudo pfctl -a notes -f /etc/pf.anchors/notes
sudo pfctl -e
```

### 9.3 Fail2Ban (or macOS Equivalent)

Since this is macOS, use a lightweight alternative. Create a script that
monitors Caddy logs for brute-force attempts:

Create `/Users/notesapp/scripts/ban-bruteforce.sh`:

```bash
#!/bin/bash
# Simple brute-force detection for macOS
# Monitors Caddy access logs for excessive POST /login attempts

LOG="/Users/notesapp/logs/caddy-access.log"
THRESHOLD=10  # Max login attempts per minute per IP
BAN_DURATION=3600  # 1 hour

while true; do
    # Find IPs with too many login attempts in the last minute
    OFFENDERS=$(grep '"POST /login"' "$LOG" | \
        grep "$(date -v-1M '+%Y-%m-%d')" | \
        jq -r '.request.remote_ip' 2>/dev/null | \
        sort | uniq -c | sort -rn | \
        awk -v t="$THRESHOLD" '$1 > t {print $2}')

    for ip in $OFFENDERS; do
        # Check if already blocked
        if ! sudo pfctl -t bruteforce -T show 2>/dev/null | grep -q "$ip"; then
            echo "$(date): Blocking $ip for brute-force" >> /Users/notesapp/logs/bans.log
            sudo pfctl -t bruteforce -T add "$ip"
        fi
    done

    sleep 60
done
```

Add to the pf config:

```
table <bruteforce> persist
block in quick from <bruteforce>
```

---

## 10. Monitoring and Alerting

### 10.1 Health Check Script

Create `/Users/notesapp/scripts/health-check.sh`:

```bash
#!/bin/bash
# Check that the notes app is responding

URL="https://yournotes.duckdns.org/"
EXPECTED=200

STATUS=$(curl -s -o /dev/null -w "%{http_code}" --max-time 10 "$URL")

if [ "$STATUS" != "$EXPECTED" ]; then
    echo "$(date): ALERT - Notes app returned $STATUS (expected $EXPECTED)" \
        >> /Users/notesapp/logs/alerts.log

    # Restart container
    cd /Users/notesapp/notes
    docker compose -f docker-compose.prod.yml restart

    # Optional: Send notification (via ntfy.sh, pushover, etc.)
    # curl -d "Notes app down! Status: $STATUS" https://ntfy.sh/your-topic
fi
```

Schedule it every 5 minutes via launchd.

### 10.2 Disk Space Monitoring

```bash
#!/bin/bash
# Alert if disk usage exceeds 80%
USAGE=$(df -h / | awk 'NR==2 {print $5}' | tr -d '%')
if [ "$USAGE" -gt 80 ]; then
    echo "$(date): ALERT - Disk usage at ${USAGE}%" \
        >> /Users/notesapp/logs/alerts.log
fi
```

### 10.3 Container Resource Monitoring

```bash
# Check container stats
docker stats --no-stream notes-app --format \
    "CPU: {{.CPUPerc}}, MEM: {{.MemUsage}}, NET: {{.NetIO}}"
```

### 10.4 Log Rotation

Caddy and Docker both handle their own log rotation (configured above). For
custom logs:

```bash
# Add to crontab
0 0 * * * find /Users/notesapp/logs -name "*.log" -size +50M \
    -exec gzip {} \; 2>/dev/null
0 0 * * 0 find /Users/notesapp/logs -name "*.gz" -mtime +30 \
    -delete 2>/dev/null
```

---

## 11. Penetration Testing

### 11.1 Automated Security Scanning (Pre-Deployment)

Run these before every deployment:

#### Dependency Audit

```bash
# Check for known vulnerable Rust dependencies
cargo install cargo-audit
cargo audit

# Run this in CI or before every deployment
```

#### OWASP ZAP Baseline Scan

```bash
# Run OWASP ZAP against your local instance
docker run -t zaproxy/zap-stable zap-baseline.py \
    -t http://host.docker.internal:3000 \
    -r zap-report.html \
    -c zap-config.conf

# For the deployed version (after Caddy is up):
docker run -t zaproxy/zap-stable zap-baseline.py \
    -t https://yournotes.duckdns.org \
    -r zap-report.html
```

#### SSL/TLS Testing

```bash
# Test TLS configuration
docker run --rm -ti drwetter/testssl.sh https://yournotes.duckdns.org

# Or use the online tool: https://www.ssllabs.com/ssltest/
```

#### Nikto Web Scanner

```bash
docker run --rm sullo/nikto -h https://yournotes.duckdns.org
```

### 11.2 Manual Penetration Testing Checklist

Run these tests manually after deployment and after every significant code
change:

```
[ ] Authentication bypass
    - Try accessing /note/*, /api/note/*, /new without session cookie
    - Try accessing with expired session token
    - Try accessing with tampered session token (change 1 byte of signature)
    - Try empty password when NOTES_PASSWORD is set

[ ] Path traversal
    - POST /new with filename: "../../../etc/passwd"
    - POST /new with filename: "/etc/passwd"
    - POST /new with filename: "....//....//etc/passwd"
    - POST /new with filename containing null byte: "test%00.md"
    - POST /api/pdf/upload with traversal filename

[ ] XSS
    - Create a note with content: <script>alert(1)</script>
    - Create a note with title: <img src=x onerror=alert(1)>
    - Search for: <script>alert(1)</script>
    - Check that all reflected values are escaped

[ ] CSRF
    - From a different origin, try to POST to /api/note/{key}
    - Verify SameSite=Strict prevents cross-origin cookie sending

[ ] Command injection
    - Create note with title containing shell metacharacters: `$(whoami)`
    - Smart-add with URL containing backticks, $(), etc.

[ ] Rate limiting
    - Send 100 POST /login requests in 10 seconds
    - Verify that rate limiting kicks in (429 or block)

[ ] Session security
    - Verify cookie has Secure flag (check in browser DevTools)
    - Verify cookie has HttpOnly flag
    - Verify cookie has SameSite=Strict
    - Verify session expires after 24 hours

[ ] TLS
    - Verify HTTPS redirect works
    - Verify HSTS header is present
    - Verify certificate is valid and auto-renews

[ ] Headers
    - Verify Content-Security-Policy header
    - Verify X-Frame-Options: DENY
    - Verify X-Content-Type-Options: nosniff
    - Verify no Server header leaks software version
```

### 11.3 Automated Pen Test Script

Create `/Users/notesapp/scripts/security-test.sh`:

```bash
#!/bin/bash
# Automated security regression tests
# Run after every deployment

BASE_URL="${1:-https://yournotes.duckdns.org}"
PASS=0
FAIL=0

check() {
    local desc="$1"
    local result="$2"
    if [ "$result" = "true" ]; then
        echo "  [PASS] $desc"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] $desc"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Security Regression Tests ==="
echo "Target: $BASE_URL"
echo ""

# 1. HTTPS redirect
echo "--- TLS ---"
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" -L "http://${BASE_URL#https://}/")
check "HTTP redirects to HTTPS" "$([ "$HTTP_STATUS" = "200" ] && echo true || echo false)"

# 2. Security headers
echo "--- Security Headers ---"
HEADERS=$(curl -s -D - -o /dev/null "$BASE_URL/")
check "HSTS header present" "$(echo "$HEADERS" | grep -qi 'strict-transport-security' && echo true || echo false)"
check "X-Frame-Options present" "$(echo "$HEADERS" | grep -qi 'x-frame-options' && echo true || echo false)"
check "X-Content-Type-Options present" "$(echo "$HEADERS" | grep -qi 'x-content-type-options' && echo true || echo false)"
check "CSP header present" "$(echo "$HEADERS" | grep -qi 'content-security-policy' && echo true || echo false)"
check "No Server header leak" "$(echo "$HEADERS" | grep -qi '^server:' && echo false || echo true)"

# 3. Authentication enforcement
echo "--- Authentication ---"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/new")
check "GET /new requires auth (redirects)" "$([ "$STATUS" = "302" ] || [ "$STATUS" = "303" ] && echo true || echo false)"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/note/test123")
check "POST /api/note requires auth" "$([ "$STATUS" = "302" ] || [ "$STATUS" = "303" ] || [ "$STATUS" = "401" ] && echo true || echo false)"

# 4. Cookie flags
echo "--- Cookie Security ---"
COOKIE_HEADER=$(curl -s -D - -o /dev/null -X POST \
    -d "password=wrong" "$BASE_URL/login" | grep -i set-cookie || echo "none")
# Wrong password should NOT set a cookie
check "No cookie on failed login" "$(echo "$COOKIE_HEADER" | grep -qi 'notes_session' && echo false || echo true)"

# 5. Path traversal
echo "--- Path Traversal ---"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/note/..%2F..%2Fetc%2Fpasswd")
check "Path traversal blocked (..)" "$([ "$STATUS" != "200" ] && echo true || echo false)"

# 6. Sensitive files
echo "--- Information Disclosure ---"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/.env")
check ".env not accessible" "$([ "$STATUS" = "404" ] || [ "$STATUS" = "403" ] && echo true || echo false)"

STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/.git/config")
check ".git not accessible via web" "$([ "$STATUS" = "404" ] || [ "$STATUS" = "403" ] && echo true || echo false)"

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] && exit 0 || exit 1
```

```bash
chmod +x /Users/notesapp/scripts/security-test.sh
```

### 11.4 Continuous Security Monitoring

Schedule the security test to run daily:

```bash
# Add to crontab
0 6 * * * /Users/notesapp/scripts/security-test.sh >> /Users/notesapp/logs/security-tests.log 2>&1
```

### 11.5 Dependency Scanning Schedule

```bash
# Weekly cargo audit
0 0 * * 1 cd /Users/notesapp/notes && cargo audit >> /Users/notesapp/logs/cargo-audit.log 2>&1
```

---

## 12. Backup Strategy

### 12.1 Git-Based Backup (Built-In)

The notes are already in git. Push to a private remote regularly:

```bash
# Host-side cron job (runs outside Docker)
*/30 * * * * cd /Users/notesapp/notes && \
    git add content/ pdfs/ && \
    git commit -m "auto-save: $(date '+\%Y-\%m-\%d \%H:\%M')" 2>/dev/null; \
    git push origin main 2>/dev/null || true
```

### 12.2 Full Backup

```bash
#!/bin/bash
# /Users/notesapp/scripts/backup.sh
BACKUP_DIR="/Users/notesapp/backups"
DATE=$(date +%Y%m%d-%H%M%S)
mkdir -p "$BACKUP_DIR"

# Create encrypted backup
tar czf - -C /Users/notesapp/notes content/ pdfs/ | \
    openssl enc -aes-256-cbc -salt -pbkdf2 \
    -out "$BACKUP_DIR/notes-$DATE.tar.gz.enc"

# Keep only last 30 days
find "$BACKUP_DIR" -name "*.enc" -mtime +30 -delete

echo "$(date): Backup completed: notes-$DATE.tar.gz.enc"
```

### 12.3 Restore

```bash
openssl enc -d -aes-256-cbc -pbkdf2 \
    -in /Users/notesapp/backups/notes-YYYYMMDD-HHMMSS.tar.gz.enc | \
    tar xzf - -C /Users/notesapp/notes/
```

---

## 13. Maintenance Runbook

### 13.1 Updating the Application

```bash
cd /Users/notesapp/notes
git pull
cargo audit                                    # Check for vulnerabilities
docker compose -f docker-compose.prod.yml build
docker compose -f docker-compose.prod.yml up -d
/Users/notesapp/scripts/security-test.sh       # Verify security posture
```

### 13.2 Renewing TLS Certificate

Caddy handles this automatically. Certificates auto-renew ~30 days before
expiry. Verify with:

```bash
echo | openssl s_client -connect yournotes.duckdns.org:443 2>/dev/null | \
    openssl x509 -noout -dates
```

### 13.3 Viewing Logs

```bash
# Application logs
docker logs notes-app --tail 100

# Caddy access logs
tail -f /Users/notesapp/logs/caddy-access.log | jq .

# Security test results
cat /Users/notesapp/logs/security-tests.log

# Failed login attempts (in Caddy logs)
grep '"POST /login"' /Users/notesapp/logs/caddy-access.log | \
    jq 'select(.status != 302)'
```

### 13.4 Emergency Procedures

**App compromised / suspicious activity:**

```bash
# 1. Immediately stop the container
docker compose -f docker-compose.prod.yml down

# 2. Block all traffic
sudo pfctl -t bruteforce -T add 0.0.0.0/0

# 3. Check git log for unauthorized changes
cd /Users/notesapp/notes
git log --oneline -20
git diff HEAD~5

# 4. Check for modified files outside git
find /Users/notesapp/notes -newer /Users/notesapp/notes/.git/index \
    -not -path '*/\.git/*'

# 5. Rotate the password
security delete-generic-password -a notesapp -s notes-password
security add-generic-password -a notesapp -s notes-password -w

# 6. Restart with new password
docker compose -f docker-compose.prod.yml up -d
```

**Container won't start:**

```bash
docker compose -f docker-compose.prod.yml logs
docker compose -f docker-compose.prod.yml down
docker compose -f docker-compose.prod.yml up -d
```

**DuckDNS not updating:**

```bash
# Manual update
/Users/notesapp/duckdns/duck.sh
cat /Users/notesapp/duckdns/duck.log  # Should say "OK"

# Check your public IP
curl -s https://api.ipify.org
```

---

## Summary: Deployment Checklist

```
Pre-deployment:
[ ] Apply all 7 code fixes from Section 2
[ ] Run cargo audit — zero vulnerabilities
[ ] Run cargo test — all tests pass
[ ] Build Docker image successfully

Infrastructure:
[ ] DuckDNS subdomain registered and updating
[ ] Router forwards port 443 only to Mac Mini
[ ] macOS firewall enabled (stealth mode)
[ ] pf rules loaded
[ ] Caddy installed with DuckDNS plugin
[ ] Caddyfile configured with security headers
[ ] Caddy running as launchd service
[ ] TLS certificate issued (check with openssl s_client)

Application:
[ ] .env file created with strong password (chmod 600)
[ ] docker-compose.prod.yml used (not docker-compose.yml)
[ ] Container running and healthy
[ ] .git mounted as read-only
[ ] Auto-save cron job configured on host

Security verification:
[ ] security-test.sh passes all checks
[ ] OWASP ZAP baseline scan — no high-severity findings
[ ] testssl.sh — grade A or better
[ ] Manual penetration test checklist completed

Monitoring:
[ ] Health check script running every 5 minutes
[ ] Security tests running daily
[ ] cargo audit running weekly
[ ] Log rotation configured
[ ] Backup script running daily
```
