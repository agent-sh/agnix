//! Gemini CLI extension manifest (`gemini-extension.json`) helpers.
//!
//! Mirrors what Gemini CLI does when it loads a manifest
//! (packages/cli/src/config/extension-manager.ts, `loadExtensionConfig`,
//! `validateName`, `validateContextFilePath`) and what
//! `gemini extensions validate` adds (commands/extensions/validate.ts).

/// `validateName`: `/^[a-zA-Z0-9-]+$/`, otherwise "Invalid extension name:
/// ... Only letters (a-z, A-Z), numbers (0-9), and dashes (-) are allowed."
pub fn is_loadable_extension_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// `gemini extensions validate` warns when `semver.valid(version)` fails.
/// node-semver trims the string and accepts one leading `v`.
pub fn is_semver(version: &str) -> bool {
    let version = version.trim();
    let version = version.strip_prefix('v').unwrap_or(version);
    version.len() <= 256 && semver::Version::parse(version).is_ok()
}

/// Words that name a secret in an extension setting's `envVar` or `name`
/// (geminicli.com/docs/extensions/best-practices: "If your extension
/// requires API keys or other secrets, use the `sensitive: true` option").
const SECRET_WORDS: &[&str] = &[
    "KEY",
    "KEYS",
    "APIKEY",
    "TOKEN",
    "TOKENS",
    "SECRET",
    "SECRETS",
    "PASSWORD",
    "PASSWORDS",
    "PASSWD",
    "CREDENTIAL",
    "CREDENTIALS",
];

/// A word after a secret word that makes the setting describe the secret
/// rather than hold it (`KEY_PATH`, `TOKEN_URL`, `ACCESS_KEY_ID`).
const NOT_SECRET_AFTER: &[&str] = &[
    "PATH", "FILE", "DIR", "URL", "URI", "ID", "NAME", "TYPE", "LIMIT", "COUNT", "SIZE", "LENGTH",
    "HEADER", "FIELD", "PREFIX", "ENDPOINT", "HOST", "PORT", "ENV", "VAR", "FORMAT", "MODE",
];

/// A word before a secret word that makes it something else
/// (`PUBLIC_KEY`, `MAX_TOKENS`).
const NOT_SECRET_BEFORE: &[&str] = &["PUBLIC", "MAX", "MIN", "NUM"];

/// Upper-case words of an identifier or label: split at characters other
/// than letters and digits and at lower-to-upper case changes
/// (`MY_API_KEY`, `apiKey`, `API Key`).
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for ch in text.chars() {
        if !ch.is_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous_lower = false;
            continue;
        }
        if ch.is_uppercase() && previous_lower && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        previous_lower = ch.is_lowercase() || ch.is_ascii_digit();
        current.extend(ch.to_uppercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Whether an extension setting's `envVar` or `name` looks like it holds a
/// secret: a whole word KEY, TOKEN, SECRET, PASSWORD or CREDENTIAL (or a
/// plural), so `KEYBOARD_LAYOUT` does not match, and not one of the
/// describing forms such as `KEY_PATH`, `TOKEN_URL`, `PUBLIC_KEY` or
/// `MAX_TOKENS`. An agnix heuristic; Gemini CLI defines no list.
pub fn looks_like_secret_setting(text: &str) -> bool {
    let words = words(text);
    words.iter().enumerate().any(|(index, word)| {
        SECRET_WORDS.contains(&word.as_str())
            && !words
                .get(index + 1)
                .is_some_and(|next| NOT_SECRET_AFTER.contains(&next.as_str()))
            && !index
                .checked_sub(1)
                .and_then(|i| words.get(i))
                .is_some_and(|previous| NOT_SECRET_BEFORE.contains(&previous.as_str()))
    })
}

/// Why `validateContextFilePath` rejects a `contextFileName` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPathProblem {
    /// Not a string, empty, or containing a null byte ("expected a
    /// non-empty string.", "null bytes are not allowed.").
    NotNonEmptyString,
    /// Starts with `${extensionPath}` or `${workspacePath}`: the manifest is
    /// hydrated before validation, so the entry becomes an absolute path.
    VariableAbsolute,
    /// POSIX or Windows absolute path ("Absolute paths are not allowed.").
    Absolute,
    /// Contains `..`, also URL-encoded ("Relative parent directory
    /// references ("..") are not allowed."). The loader checks the
    /// substring, so `notes..md` is rejected too.
    ParentReference,
}

/// Check one `contextFileName` entry the way the loader does, after the
/// `${/}` / `${pathSeparator}` hydration. Returns the hydrated path when it
/// is accepted.
pub fn check_context_file_path(entry: &str) -> Result<String, ContextPathProblem> {
    if entry.trim().is_empty() || entry.contains('\0') {
        return Err(ContextPathProblem::NotNonEmptyString);
    }
    if entry.starts_with("${extensionPath}") || entry.starts_with("${workspacePath}") {
        return Err(ContextPathProblem::VariableAbsolute);
    }
    let hydrated = entry.replace("${/}", "/").replace("${pathSeparator}", "/");
    if is_absolute_posix_or_windows(&hydrated) {
        return Err(ContextPathProblem::Absolute);
    }
    if hydrated.contains("..") || percent_decode(&hydrated).contains("..") {
        return Err(ContextPathProblem::ParentReference);
    }
    Ok(hydrated)
}

/// `path.isAbsolute(p) || path.win32.isAbsolute(p)`.
fn is_absolute_posix_or_windows(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || path.starts_with('\\')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

/// Decode `%XX` escapes (like `decodeURIComponent`, keeping malformed ones).
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_settings_match_whole_words() {
        for secret in [
            "MY_API_KEY",
            "API Key",
            "apiKey",
            "GITHUB_TOKEN",
            "Access tokens",
            "CLIENT_SECRET",
            "AWS_SECRET_ACCESS_KEY",
            "DB_PASSWORD",
            "Password",
            "SERVICE_CREDENTIALS",
            "OPENAI_APIKEY",
        ] {
            assert!(looks_like_secret_setting(secret), "{secret}");
        }
        for other in [
            "KEYBOARD_LAYOUT",
            "MONKEY",
            "TOKENIZER",
            "SSH_KEY_PATH",
            "KEY_FILE",
            "AWS_ACCESS_KEY_ID",
            "TOKEN_URL",
            "PUBLIC_KEY",
            "Max tokens",
            "MAX_TOKENS",
            "API Base URL",
            "Project ID",
            "",
        ] {
            assert!(!looks_like_secret_setting(other), "{other}");
        }
    }

    #[test]
    fn loadable_names_follow_validate_name() {
        for name in [
            "my-extension",
            "ext",
            "ext123",
            "a",
            "0ext",
            "My-Extension",
            "-ext",
            "EXT",
        ] {
            assert!(is_loadable_extension_name(name), "{name}");
        }
        for name in ["", "ext_name", "ext name", "ext.name", "café"] {
            assert!(!is_loadable_extension_name(name), "{name}");
        }
    }

    #[test]
    fn semver_matches_node_semver_valid() {
        for version in [
            "1.0.0",
            "v1.2.3",
            " 1.2.3 ",
            "1.0.0-beta.1",
            "1.0.0+build.5",
        ] {
            assert!(is_semver(version), "{version}");
        }
        for version in ["1.0", "1", "latest", "01.0.0", "1.0.0.0", ""] {
            assert!(!is_semver(version), "{version}");
        }
    }

    #[test]
    fn context_file_paths_follow_validate_context_file_path() {
        use ContextPathProblem::*;
        for (entry, expected) in [
            ("GEMINI.md", Ok("GEMINI.md".to_string())),
            ("docs/CONTEXT.md", Ok("docs/CONTEXT.md".to_string())),
            ("docs${/}CONTEXT.md", Ok("docs/CONTEXT.md".to_string())),
            ("./GEMINI.md", Ok("./GEMINI.md".to_string())),
            ("", Err(NotNonEmptyString)),
            ("  ", Err(NotNonEmptyString)),
            ("a\0b", Err(NotNonEmptyString)),
            ("${extensionPath}/GEMINI.md", Err(VariableAbsolute)),
            ("/etc/GEMINI.md", Err(Absolute)),
            ("C:\\ctx\\GEMINI.md", Err(Absolute)),
            ("C:/ctx/GEMINI.md", Err(Absolute)),
            ("\\\\server\\share\\GEMINI.md", Err(Absolute)),
            ("../shared/GEMINI.md", Err(ParentReference)),
            ("docs/../GEMINI.md", Err(ParentReference)),
            ("notes..md", Err(ParentReference)),
            ("%2e%2e/GEMINI.md", Err(ParentReference)),
        ] {
            assert_eq!(check_context_file_path(entry), expected, "{entry:?}");
        }
    }
}
