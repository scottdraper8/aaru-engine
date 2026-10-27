use regex::Regex;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::LazyLock;

static PACKAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]*$").unwrap());
static VERSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9][A-Za-z0-9.!+_-]*$").unwrap());

fn packages(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut packages = Vec::new();
    let mut names = BTreeSet::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') || line.starts_with("--hash=sha256:") {
            continue;
        }
        let line = line.trim_end_matches('\\').trim();
        let (name, version) = line
            .split_once("==")
            .ok_or("framework requirements must use exact pins")?;
        if !PACKAGE.is_match(name) || !VERSION.is_match(version) {
            return Err(
                "unsupported framework requirement; ranges/URLs/wildcards are not allowed".into(),
            );
        }
        let normalized = name.to_ascii_lowercase().replace(['_', '.'], "-");
        if !names.insert(normalized.clone()) {
            return Err(format!("duplicate framework requirement {normalized}"));
        }
        packages.push((normalized, version.to_owned()));
    }
    if packages.is_empty() || packages.len() > 128 {
        return Err("framework requirements must contain 1 to 128 packages".into());
    }
    Ok(packages)
}

fn findings(packages: &[(String, String)], response: &str) -> Result<Vec<String>, String> {
    if response.len() > 2_097_152 {
        return Err("oversized OSV response".into());
    }
    let response: Value = serde_json::from_str(response).map_err(|error| error.to_string())?;
    let results = response
        .get("results")
        .and_then(Value::as_array)
        .ok_or("OSV response lacks results")?;
    if response.get("error").is_some() || results.len() != packages.len() {
        return Err("OSV response does not match the complete query".into());
    }
    let mut errors = Vec::new();
    for ((name, version), result) in packages.iter().zip(results) {
        if !result.is_object() || result.get("error").is_some() {
            return Err("OSV returned an invalid package result".into());
        }
        if let Some(vulns) = result.get("vulns") {
            for vuln in vulns.as_array().ok_or("invalid OSV advisory list")? {
                let id = vuln
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|id| {
                        !id.is_empty()
                            && id.len() <= 128
                            && id
                                .bytes()
                                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                    })
                    .ok_or("invalid OSV advisory identity")?;
                errors.push(format!("framework {name}=={version}: {id}"));
            }
        }
    }
    Ok(errors)
}

pub fn audit(path: &Path) -> Result<Vec<String>, String> {
    let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > 524_288 {
        return Err("oversized framework requirements".into());
    }
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let packages = packages(&text)?;
    let queries: Vec<_> = packages.iter().map(|(name, version)| {
        json!({"package": {"name": name, "ecosystem": "PyPI"}, "version": version})
    }).collect();
    let request =
        serde_json::to_vec(&json!({"queries": queries})).map_err(|error| error.to_string())?;
    let mut child = Command::new("curl")
        .args([
            "--disable",
            "--proto",
            "=https",
            "--connect-timeout",
            "10",
            "--max-time",
            "30",
            "--max-filesize",
            "2097152",
            "--fail",
            "--silent",
            "--show-error",
            "--header",
            "Content-Type: application/json",
            "--data-binary",
            "@-",
            "https://api.osv.dev/v1/querybatch",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("start OS-managed HTTPS transport: {error}"))?;
    let written = child
        .stdin
        .take()
        .ok_or_else(|| "missing request pipe".to_owned())
        .and_then(|mut input| input.write_all(&request).map_err(|error| error.to_string()));
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    written?;
    if !output.status.success() {
        return Err(format!(
            "OSV request failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let response = std::str::from_utf8(&output.stdout).map_err(|error| error.to_string())?;
    findings(&packages, response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_framework_versions_are_not_assumed_to_be_cargo_semver() {
        let input = "pre-commit==4.6.2 \\\n    --hash=sha256:abc\npackaging==26.3\nPython_Discovery==1.6.5\n";
        assert_eq!(
            packages(input).unwrap(),
            vec![
                ("pre-commit".into(), "4.6.2".into()),
                ("packaging".into(), "26.3".into()),
                ("python-discovery".into(), "1.6.5".into()),
            ]
        );
        for invalid in [
            "",
            "a>=1.0",
            "a==1.*",
            "https://example.test/package.whl",
            "a==1.0\nA==2.0\n",
        ] {
            assert!(packages(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn all_advisories_fail_even_without_severity_metadata() {
        let packages = vec![("requests".into(), "2.19.1".into())];
        assert_eq!(
            findings(
                &packages,
                r#"{"results":[{"vulns":[{"id":"GHSA-x84v-xcm2-53pg"}]}]}"#
            )
            .unwrap(),
            vec!["framework requests==2.19.1: GHSA-x84v-xcm2-53pg"]
        );
        assert!(
            findings(&packages, r#"{"results":[{}]}"#)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn incomplete_or_malformed_advisory_responses_never_mean_clean() {
        let packages = vec![("pre-commit".into(), "4.6.2".into())];
        for malformed in [
            "not json",
            "{}",
            r#"{"results":[]}"#,
            r#"{"results":[{},{}]}"#,
            r#"{"results":[null]}"#,
            r#"{"results":[{"error":"unavailable"}]}"#,
            r#"{"results":[{"vulns":null}]}"#,
            r#"{"results":[{"vulns":[{}]}]}"#,
        ] {
            assert!(findings(&packages, malformed).is_err(), "{malformed}");
        }
    }
}
