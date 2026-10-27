use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

static PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"-{4,5} ?BEGIN (?:[A-Z0-9]+ )*PRIVATE KEY(?: BLOCK)? ?-{4,5}|PuTTY-User-Key-File-[123]:",
    )
    .unwrap()
});

pub fn is_text(name: &str) -> bool {
    let path = Path::new(name);
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some(
            "rs" | "toml"
                | "json"
                | "yml"
                | "yaml"
                | "md"
                | "html"
                | "css"
                | "svg"
                | "txt"
                | "in"
                | "lock"
                | "py"
                | "js"
                | "mjs"
                | "ts"
                | "tsx"
                | "jsx"
                | "sh"
                | "bash"
                | "ps1"
                | "bat"
                | "cmd"
                | "case"
                | "asc"
                | "ppk"
        )
    ) || matches!(
        path.file_name().and_then(|file| file.to_str()),
        Some(
            ".gitignore"
                | ".gitattributes"
                | ".agent-deny"
                | "NOTICE"
                | ".env.example"
                | ".env.sample"
                | ".env.template"
        )
    )
}

pub fn portable_paths<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut seen = BTreeMap::new();
    let mut errors = Vec::new();
    for name in names {
        let mut prefix = String::new();
        for component in name.split('/') {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            let folded = prefix.to_lowercase();
            if let Some(previous) = seen.insert(folded, prefix.clone())
                && previous != prefix
            {
                errors.push(format!(
                    "{name}: case-conflicting path {previous} / {prefix}"
                ));
            }
            let stem = component
                .split('.')
                .next()
                .unwrap_or("")
                .to_ascii_uppercase();
            let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                || ["COM", "LPT"].iter().any(|base| {
                    stem.strip_prefix(base)
                        .is_some_and(|n| n.len() == 1 && matches!(n.as_bytes()[0], b'1'..=b'9'))
                });
            if device
                || component.ends_with(['.', ' '])
                || component.chars().any(|c| {
                    c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*')
                })
            {
                errors.push(format!("{name}: path is not portable to Windows"));
            }
        }
    }
    errors
}

pub fn metadata(name: &str, mode: &str, size: u64) -> Vec<String> {
    let mut errors = Vec::new();
    if !matches!(mode, "100644" | "100755") {
        errors.push(format!(
            "{name}: symlinks, submodules and unusual file modes are not allowed"
        ));
    }
    if size > MAX_FILE_BYTES {
        errors.push(format!("{name}: exceeds the 2048 KiB tracked-file limit"));
    }
    errors
}

pub fn inspect_secrets(name: &str, text: &str) -> Vec<String> {
    if PRIVATE_KEY.is_match(text) {
        vec![format!("{name}: private-key material must not be tracked")]
    } else {
        Vec::new()
    }
}

pub fn inspect(name: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let mut fail = |message: String| errors.push(format!("{name}: {message}"));
    if text.contains('\r') || text.contains('\0') {
        fail("text must use LF newlines without NUL bytes".into());
    }
    if !text.is_empty() && (!text.ends_with('\n') || text.ends_with("\n\n")) {
        fail("nonempty text must end with exactly one newline".into());
    }
    for (index, line) in text.lines().enumerate() {
        if line.ends_with([' ', '\t']) {
            fail(format!("line {} has trailing whitespace", index + 1));
        }
        if ["<<<<<<<", ">>>>>>>", "|||||||"].iter().any(|marker| {
            line == *marker
                || line
                    .strip_prefix(marker)
                    .is_some_and(|rest| rest.starts_with(' '))
        }) {
            fail(format!(
                "line {} contains a merge-conflict marker",
                index + 1
            ));
        }
    }
    let path = Path::new(name);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("json") => {
            if let Err(error) = serde_json::from_str::<serde_json::Value>(text) {
                fail(format!("invalid JSON: {error}"));
            }
        }
        Some("toml") if path.file_name().and_then(|file| file.to_str()) != Some("Cargo.toml") => {
            if let Err(error) = text.parse::<toml::Table>() {
                fail(format!("invalid TOML: {error}"));
            }
        }
        Some("yml" | "yaml")
            if !name.starts_with(".github/workflows/") && name != ".pre-commit-config.yaml" =>
        {
            if let Err(error) = serde_saphyr::from_str::<serde_json::Value>(text) {
                fail(format!("invalid YAML: {error}"));
            }
        }
        _ => {}
    }
    errors.extend(inspect_secrets(name, text));
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracked_file_size_is_checked_at_the_actual_boundary() {
        assert!(metadata("fixture.json", "100644", 2_097_152).is_empty());
        assert!(!metadata("fixture.json", "100644", 2_097_153).is_empty());
        for mode in ["120000", "160000", "000000"] {
            assert!(!metadata("link.rs", mode, 12).is_empty());
        }
    }

    #[test]
    fn portability_checks_directory_collisions_and_windows_devices() {
        assert!(portable_paths(["src/main.rs", "src/lib.rs", "fixtures/梦.json"]).is_empty());
        assert!(!portable_paths(["src/one.rs", "Src/two.rs"]).is_empty());
        assert!(!portable_paths(["README.md", "readme.md"]).is_empty());
        for path in [
            "con.txt",
            "fixtures/AUX",
            "a/COM1.json",
            "LPT9.md",
            "a./b.rs",
            "a:b.rs",
            "a\\b.rs",
        ] {
            assert!(!portable_paths([path]).is_empty(), "{path}");
        }
    }

    #[test]
    fn text_integrity_checks_preserve_valid_unicode_and_markdown_headings() {
        assert!(inspect("notes.md", "梦想\n=======\n").is_empty());
        assert!(inspect("empty.rs", "").is_empty());
        for text in [
            "no newline",
            "two\n\n",
            "spaces \n",
            "tabs\t\n",
            "windows\r\n",
            "nul\0\n",
            "<<<<<<< ours\n",
            ">>>>>>> theirs\n",
            "||||||| base\n",
        ] {
            assert!(!inspect("source.rs", text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn format_validation_is_not_limited_to_one_configuration_filename() {
        for (path, valid, invalid) in [
            ("fixtures/data.json", "{\"level\": 90}\n", "{\"level\":}\n"),
            (
                "deny.toml",
                "[licenses]\nallow = [\"MIT\"]\n",
                "[licenses\n",
            ),
            (
                ".github/dependabot.yml",
                "version: 2\nupdates: []\n",
                "updates: [\n",
            ),
        ] {
            assert!(inspect(path, valid).is_empty(), "{path}");
            assert!(!inspect(path, invalid).is_empty(), "{path}");
        }
    }

    #[test]
    fn keys_are_detected_without_storing_any_secret_key_fixture() {
        for algorithm in ["RSA", "EC", "OPENSSH", "ENCRYPTED"] {
            let header = format!("-----BEGIN {algorithm} PRIVATE KEY-----\n");
            assert!(!inspect("accidental.txt", &header).is_empty());
        }
        assert!(
            !inspect(
                "accidental.txt",
                &format!("PuTTY-User-Key-File-{}: ssh-ed25519\n", 3)
            )
            .is_empty()
        );
        for header in [
            format!("-----BEGIN PGP PRIVATE KEY {}-----\n", "BLOCK"),
            format!("---- BEGIN {} PRIVATE KEY ----\n", "SSH2 ENCRYPTED"),
        ] {
            assert!(!inspect("accidental.txt", &header).is_empty());
        }
        assert!(inspect("public.txt", "Public key documentation.\n").is_empty());
    }
}
