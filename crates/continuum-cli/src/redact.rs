//! Secret redaction for safe LLM interactions.
//!
//! Before sending code or text to an LLM, redact sensitive patterns
//! like API keys, JWTs, PEM keys, database URIs, etc.
//! Based on patterns from Gokin (24 regex patterns).

use regex::Regex;
use std::sync::OnceLock;

/// A redaction pattern with a name and regex.
struct RedactionPattern {
    name: &'static str,
    regex: Regex,
}

/// Static storage for compiled patterns.
static PATTERNS: OnceLock<Vec<RedactionPattern>> = OnceLock::new();

/// Get all redaction patterns (compiled once, cached forever).
fn patterns() -> &'static Vec<RedactionPattern> {
    PATTERNS.get_or_init(|| {
        vec![
            // API Keys
            RedactionPattern {
                name: "generic_api_key",
                regex: Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*["']?([A-Za-z0-9\-_.]{20,})["']?"#).unwrap(),
            },
            RedactionPattern {
                name: "bearer_token",
                regex: Regex::new(r#"(?i)(bearer)\s+[A-Za-z0-9\-_.]{20,}"#).unwrap(),
            },
            RedactionPattern {
                name: "authorization_header",
                regex: Regex::new(r#"(?i)(authorization)\s*[:=]\s*["']?([A-Za-z0-9\-_.: ]{20,})["']?"#).unwrap(),
            },

        // Provider-specific keys
        RedactionPattern {
            name: "anthropic_key",
            regex: Regex::new(r"sk-ant-[A-Za-z0-9\-_]{20,}").unwrap(),
        },
        RedactionPattern {
            name: "openai_key",
            regex: Regex::new(r"sk-[A-Za-z0-9\-]{20,}").unwrap(),
        },
        RedactionPattern {
            name: "openrouter_key",
            regex: Regex::new(r"sk-or-[A-Za-z0-9\-_]{20,}").unwrap(),
        },
        RedactionPattern {
            name: "github_token",
            regex: Regex::new(r"gh[ps]_[A-Za-z0-9_]{36,}").unwrap(),
        },
        RedactionPattern {
            name: "github_fine_grained",
            regex: Regex::new(r"github_pat_[A-Za-z0-9_]{22,}").unwrap(),
        },
        RedactionPattern {
            name: "aws_access_key",
            regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
        },
        RedactionPattern {
            name: "aws_secret_key",
            regex: Regex::new(r#"(?i)(aws[_-]?secret[_-]?access[_-]?key)\s*[:=]\s*["']?([A-Za-z0-9/+=]{40})["']?"#).unwrap(),
        },
        RedactionPattern {
            name: "google_api_key",
            regex: Regex::new(r"AIza[0-9A-Za-z\-_]{35}").unwrap(),
        },
        RedactionPattern {
            name: "groq_key",
            regex: Regex::new(r"gsk_[A-Za-z0-9]{40,}").unwrap(),
        },
        RedactionPattern {
            name: "stripe_key",
            regex: Regex::new(r"sk_(live|test)_[A-Za-z0-9]{24,}").unwrap(),
        },

        // JWTs
        RedactionPattern {
            name: "jwt",
            regex: Regex::new(r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_.]+").unwrap(),
        },

        // PEM keys
        RedactionPattern {
            name: "private_key_pem",
            regex: Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
        },

        // Database URIs
        RedactionPattern {
            name: "database_uri",
            regex: Regex::new(r#"(?i)(postgres(ql)?|mysql|mongodb|redis|sqlite)://[^\s"']+"#).unwrap(),
        },

        // Connection strings
        RedactionPattern {
            name: "connection_string",
            regex: Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*["']?[^\s"']{8,}["']?"#).unwrap(),
        },

        // Generic secrets
        RedactionPattern {
            name: "generic_secret",
            regex: Regex::new(r#"(?i)(secret|token|password|credential)\s*[:=]\s*["']([A-Za-z0-9\-_.]{16,})["']"#).unwrap(),
        },

        // IP addresses with ports (potential internal endpoints)
        RedactionPattern {
            name: "internal_endpoint",
            regex: Regex::new(r#"(?i)(endpoint|host|url)\s*[:=]\s*["']?https?://\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}:\d+["']?"#).unwrap(),
        },

        // SSH private keys
        RedactionPattern {
            name: "ssh_key",
            regex: Regex::new(r"-----BEGIN OPENSSH PRIVATE KEY-----[\s\S]*?-----END OPENSSH PRIVATE KEY-----").unwrap(),
        },

        // PGP keys
        RedactionPattern {
            name: "pgp_key",
            regex: Regex::new(r"-----BEGIN PGP PRIVATE KEY BLOCK-----[\s\S]*?-----END PGP PRIVATE KEY BLOCK-----").unwrap(),
        },

        // Base64 encoded secrets (long strings)
        RedactionPattern {
            name: "base64_secret",
            regex: Regex::new(r#"(?i)(key|secret|token)\s*[:=]\s*["']?[A-Za-z0-9+/]{40,}={0,2}["']?"#).unwrap(),
        },

        // Webhook URLs
        RedactionPattern {
            name: "webhook_url",
            regex: Regex::new(r#"(?i)https?://[^/\s]*\.(webhook\.site|hook\.slack\.com|discord\.com/api/webhooks|api\.telegram\.org/bot)[^\s"']+"#).unwrap(),
        },

        // Environment variable assignments in code
        RedactionPattern {
            name: "env_assignment",
            regex: Regex::new(r#"(?i)(ANTHROPIC_API_KEY|OPENAI_API_KEY|GEMINI_API_KEY|GROQ_API_KEY|DEEPSEEK_API_KEY|OPENROUTER_API_KEY|MISTRAL_API_KEY)\s*=\s*["']?[^\s"']{10,}["']?"#).unwrap(),
        },
        ]
    })
}

/// Redact secrets from text, replacing them with [REDACTED:pattern_name].
///
/// Returns the redacted text and a count of redactions made.
pub fn redact(text: &str) -> (String, usize) {
    let patterns = patterns();
    let mut result = text.to_string();
    let mut total_redactions = 0;

    for pattern in patterns {
        let matches: Vec<_> = pattern
            .regex
            .find_iter(&result)
            .map(|m| m.as_str().to_string())
            .collect();
        for match_str in matches {
            let redacted = format!("[REDACTED:{}]", pattern.name);
            result = result.replace(&match_str, &redacted);
            total_redactions += 1;
        }
    }

    (result, total_redactions)
}

/// Check if text contains any secrets (without modifying it).
///
/// Returns true if secrets were found.
#[allow(dead_code)]
pub fn contains_secrets(text: &str) -> bool {
    let patterns = patterns();
    patterns.iter().any(|p| p.regex.is_match(text))
}

/// List all detected secret types in text.
///
/// Returns a list of pattern names that matched.
pub fn detect_secrets(text: &str) -> Vec<&'static str> {
    let patterns = patterns();
    patterns
        .iter()
        .filter(|p| p.regex.is_match(text))
        .map(|p| p.name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_anthropic_key() {
        let text = "api_key = sk-ant-api03-abc123def456ghi789jkl012mno345pqr678stu901vwx234";
        let (redacted, count) = redact(text);
        assert!(count > 0);
        assert!(!redacted.contains("sk-ant-"));
        assert!(redacted.contains("[REDACTED:"));
    }

    #[test]
    fn test_redact_openai_key() {
        let text = "OPENAI_API_KEY = sk-proj-abc123def456ghi789jkl012mno345pqr678stu901vwx234";
        let (redacted, count) = redact(text);
        assert!(count > 0);
        assert!(!redacted.contains("sk-proj-"));
    }



    #[test]
    fn test_redact_jwt() {
        let text = "token = eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let (redacted, count) = redact(text);
        assert!(count > 0);
        assert!(!redacted.contains("eyJhbG"));
    }

    #[test]
    fn test_redact_private_key() {
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA0Z3VS5JJcds3xfn/ygWyF8PbnGy5AH...\n-----END RSA PRIVATE KEY-----";
        let (redacted, count) = redact(text);
        assert!(count > 0);
        assert!(!redacted.contains("BEGIN RSA PRIVATE KEY"));
    }

    #[test]
    fn test_contains_secrets() {
        let clean = "fn main() { println!(\"hello\"); }";
        let dirty = "api_key = sk-ant-api03-abc123def456";
        assert!(!contains_secrets(clean));
        assert!(contains_secrets(dirty));
    }

    #[test]
    fn test_detect_secrets() {
        let text = "OPENAI_API_KEY = sk-proj-abc123def456ghi789jkl012mno345";
        let detected = detect_secrets(text);
        assert!(detected.contains(&"openai_key"));
    }
}
