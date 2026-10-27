//! Repository-specific supply-chain and check-integrity validation.

use regex::Regex;
use serde_json::Value as Yaml;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::sync::LazyLock;

static EXACT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?$").unwrap());
static SHA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-f]{40}$").unwrap());
static SUPPRESSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?m)(?://|\#|/\*|<!--)\s*(?:(?:eslint|biome|stylelint)[- ](?:disable|ignore)|",
        r"@ts-(?:ignore|nocheck)|noqa\b|nosec\b|type:\s*ignore|pre-commit:\s*skip)",
        r"|\#\s*\[\s*(?:allow|expect)\s*\(|\#\s*\[\s*ignore\b",
        r"|\#\s*\[\s*cfg_attr\b[^\]]*\b(?:allow|expect|ignore)\b",
        r"|\b(?:test|describe|it)\.(?:skip|only|todo)\s*\("
    ))
    .unwrap()
});
static BYPASS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"--no-",
        r"verify\b|\bSKIP\s*=|core\.hooksPath\s*[= ]|--cap-lints[= ]+allow"
    ))
    .unwrap()
});
static SENSITIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i)(?:^|/)(?:\.env(?:\..*)?|auth\.json|credentials\.json|",
        r"service-account[^/]*\.json|[^/]*-credentials\.json|",
        r"secrets\.(?:json|ya?ml|toml)|vault\.yaml|id_(?:rsa|ed25519|ecdsa)[^/]*|",
        r"\.netrc|\.npmrc|\.pypirc|private-data|node_modules|vendor|",
        r"\.venv|__pycache__|dist|build|out|\.idea|\.vscode|target|web/pkg|\.site)",
        r"(?:/|$)|\.(?:pem|key|p12|pfx|pyc|o|so|dylib)$"
    ))
    .unwrap()
});

fn issue(errors: &mut Vec<String>, name: &str, message: impl AsRef<str>) {
    errors.push(format!("{name}: {}", message.as_ref()));
}

fn cargo_tables(name: &str, table: &toml::Table, errors: &mut Vec<String>) {
    for (key, value) in table {
        if key.ends_with("dependencies") {
            if let Some(dependencies) = value.as_table() {
                for (dependency, specification) in dependencies {
                    if specification.as_table().is_some_and(|spec| {
                        spec.get("workspace").and_then(toml::Value::as_bool) == Some(true)
                            || (spec.contains_key("path") && !spec.contains_key("git"))
                    }) {
                        continue;
                    }
                    let version = specification
                        .as_str()
                        .or_else(|| specification.get("version").and_then(toml::Value::as_str));
                    if !version.is_some_and(|version| {
                        version.strip_prefix('=').is_some_and(|v| EXACT.is_match(v))
                    }) {
                        issue(
                            errors,
                            name,
                            format!("{dependency} needs an exact =version"),
                        );
                    }
                    if specification.get("git").is_some() {
                        issue(
                            errors,
                            name,
                            format!("{dependency}: Git dependencies are not approved"),
                        );
                    }
                }
            }
        } else if let Some(nested) = value.as_table() {
            cargo_tables(name, nested, errors);
        }
    }
}

fn lint_levels(name: &str, table: &toml::Table, errors: &mut Vec<String>) {
    for (key, value) in table {
        if value.as_str() == Some("allow")
            || value.get("level").and_then(toml::Value::as_str) == Some("allow")
        {
            issue(
                errors,
                name,
                format!("lint {key} cannot be weakened to allow"),
            );
        } else if let Some(nested) = value.as_table() {
            lint_levels(name, nested, errors);
        }
    }
}

fn permissions_are_read_only(value: &Yaml) -> bool {
    match value {
        Yaml::String(value) => value == "read-all",
        Yaml::Object(value) => value
            .values()
            .all(|v| matches!(v.as_str(), Some("read" | "none"))),
        _ => false,
    }
}

fn workflow(name: &str, document: &Yaml, errors: &mut Vec<String>) {
    let privileged_trigger = document.get("on").is_some_and(|events| {
        events.as_str() == Some("pull_request_target")
            || events.get("pull_request_target").is_some()
            || events.as_array().is_some_and(|items| {
                items
                    .iter()
                    .any(|event| event.as_str() == Some("pull_request_target"))
            })
    });
    if privileged_trigger {
        issue(
            errors,
            name,
            "untrusted pull requests must not run in a privileged trigger",
        );
    }
    if !document
        .get("permissions")
        .is_some_and(permissions_are_read_only)
    {
        issue(
            errors,
            name,
            "workflow permissions must be explicitly read-only",
        );
    }
    let Some(jobs) = document.get("jobs").and_then(Yaml::as_object) else {
        issue(errors, name, "workflow must define jobs");
        return;
    };
    if name == ".github/workflows/ci.yml" {
        let gate = jobs
            .iter()
            .find(|(_, job)| job.get("name").and_then(Yaml::as_str) == Some("Quality gate"));
        if let Some((id, gate)) = gate {
            let expected: BTreeSet<_> = jobs
                .keys()
                .filter(|other| *other != id)
                .map(String::as_str)
                .collect();
            let actual: BTreeSet<_> = gate
                .get("needs")
                .and_then(Yaml::as_array)
                .into_iter()
                .flatten()
                .filter_map(Yaml::as_str)
                .collect();
            if expected.is_empty() || actual != expected {
                issue(
                    errors,
                    name,
                    "Quality gate must depend on every other CI job",
                );
            }
            if !matches!(
                gate.get("if").and_then(Yaml::as_str),
                Some("${{ always() }}" | "always()")
            ) {
                issue(
                    errors,
                    name,
                    "Quality gate must run even when prerequisites fail",
                );
            }
            let real_gate = gate
                .get("steps")
                .and_then(Yaml::as_array)
                .is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step.get("run").and_then(Yaml::as_str).is_some_and(|run| {
                            run.trim()
                                == "cargo run --locked --quiet -p aaru-repo-policy -- ci-gate"
                        }) && step.get("if").is_none()
                            && step
                                .get("env")
                                .and_then(|env| env.get("AARU_CI_NEEDS"))
                                .and_then(Yaml::as_str)
                                == Some("${{ toJSON(needs) }}")
                    })
                });
            if !real_gate {
                issue(
                    errors,
                    name,
                    "Quality gate must evaluate actual prerequisite outcomes in Rust",
                );
            }
        } else {
            issue(errors, name, "missing required Quality gate job");
        }
    }
    for (id, job) in jobs {
        let label = format!("job {id}");
        if !job
            .get("timeout-minutes")
            .and_then(Yaml::as_u64)
            .is_some_and(|n| (1..=60).contains(&n))
        {
            issue(errors, name, format!("{label} needs a bounded timeout"));
        }
        if job
            .get("permissions")
            .is_some_and(|p| !permissions_are_read_only(p))
        {
            issue(
                errors,
                name,
                format!("{label} must not elevate token permissions"),
            );
        }
        if job.get("continue-on-error").and_then(Yaml::as_bool) == Some(true) {
            issue(errors, name, format!("{label} cannot ignore failure"));
        }
        let Some(steps) = job.get("steps").and_then(Yaml::as_array) else {
            issue(
                errors,
                name,
                format!("{label} must have explicit hardened steps"),
            );
            continue;
        };
        if !steps
            .first()
            .and_then(|step| step.get("uses"))
            .and_then(Yaml::as_str)
            .is_some_and(|action| action.starts_with("step-security/harden-runner@"))
        {
            issue(
                errors,
                name,
                format!("{label} must harden the runner as its first step"),
            );
        }
        if let Some(hardening) = steps.first() {
            if hardening.get("if").is_some() {
                issue(
                    errors,
                    name,
                    format!("{label}: runner hardening cannot be conditional"),
                );
            }
            let mode = hardening
                .get("with")
                .and_then(|options| options.get("egress-policy"))
                .and_then(Yaml::as_str);
            let runner = job.get("runs-on").and_then(Yaml::as_str).unwrap_or("");
            let valid_mode = if runner.starts_with("ubuntu-") {
                mode == Some("block")
            } else if runner.starts_with("windows-") {
                mode == Some("audit")
            } else {
                mode == Some("${{ runner.os == 'Linux' && 'block' || 'audit' }}")
            };
            if !valid_mode {
                issue(
                    errors,
                    name,
                    format!("{label}: hardening must use Linux block / Windows audit mode"),
                );
            }
        }
        for step in steps {
            if let Some(action) = step.get("uses").and_then(Yaml::as_str)
                && !action.starts_with("./")
                && !action
                    .rsplit_once('@')
                    .is_some_and(|(_, sha)| SHA.is_match(sha))
            {
                issue(
                    errors,
                    name,
                    format!("{label}: action needs an immutable SHA: {action}"),
                );
            }
            if step.get("continue-on-error").and_then(Yaml::as_bool) == Some(true) {
                issue(errors, name, format!("{label} cannot ignore step failures"));
            }
            if step
                .get("uses")
                .and_then(Yaml::as_str)
                .is_some_and(|s| s.starts_with("actions/checkout@"))
                && step
                    .get("with")
                    .and_then(|w| w.get("persist-credentials"))
                    .and_then(Yaml::as_bool)
                    != Some(false)
            {
                issue(
                    errors,
                    name,
                    format!("{label}: checkout must not persist credentials"),
                );
            }
        }
    }
}

fn inspect_text(name: &str, text: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let path = Path::new(name);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    if name == "LICENSE.md"
        && Sha256::digest(text.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            != "9eb48619fbc193ab7bb327b090cfcc703000265b83e670f81f231d0b1c43c56e"
    {
        issue(
            &mut errors,
            name,
            "must contain the exact upstream PolyForm Strict 1.0.0 license",
        );
    }
    if matches!(extension, "rs" | "js" | "mjs" | "ts" | "py") && SUPPRESSION.is_match(text) {
        issue(
            &mut errors,
            name,
            "inline suppression or disabled/focused test",
        );
    }
    if matches!(
        extension,
        "yml" | "yaml" | "sh" | "rs" | "py" | "js" | "mjs" | "json"
    ) && BYPASS.is_match(text)
    {
        issue(&mut errors, name, "executable hook or lint bypass");
    }
    if matches!(extension, "py" | "js" | "mjs" | "ts" | "tsx" | "jsx")
        || path.file_name().and_then(|s| s.to_str()) == Some("package.json")
    {
        issue(
            &mut errors,
            name,
            "first-party tooling and application code must use Rust",
        );
    }
    if path.file_name().and_then(|s| s.to_str()) == Some("Cargo.toml") {
        match text.parse::<toml::Table>() {
            Ok(table) => {
                cargo_tables(name, &table, &mut errors);
                if let Some(lints) = table.get("lints").and_then(toml::Value::as_table) {
                    lint_levels(name, lints, &mut errors);
                }
                if let Some(lints) = table
                    .get("workspace")
                    .and_then(|w| w.get("lints"))
                    .and_then(toml::Value::as_table)
                {
                    lint_levels(name, lints, &mut errors);
                }
            }
            Err(error) => issue(&mut errors, name, format!("invalid TOML: {error}")),
        }
    }
    if name == "rust-toolchain.toml" {
        let pinned = text
            .parse::<toml::Table>()
            .ok()
            .and_then(|t| {
                t.get("toolchain")?
                    .get("channel")?
                    .as_str()
                    .map(str::to_owned)
            })
            .is_some_and(|channel| EXACT.is_match(&channel));
        if !pinned {
            issue(
                &mut errors,
                name,
                "Rust toolchain must use an exact version",
            );
        }
    }
    if name.starts_with(".github/workflows/") || name == ".pre-commit-config.yaml" {
        match serde_saphyr::from_str::<Yaml>(text) {
            Ok(document) if name.starts_with(".github/workflows/") => {
                workflow(name, &document, &mut errors)
            }
            Ok(document) => {
                let Some(repos) = document.get("repos").and_then(Yaml::as_array) else {
                    issue(&mut errors, name, "missing hook repositories");
                    return errors;
                };
                let mut hooks = Vec::new();
                for repo in repos {
                    if repo.get("repo").and_then(Yaml::as_str) != Some("local")
                        && !repo
                            .get("rev")
                            .and_then(Yaml::as_str)
                            .is_some_and(|sha| SHA.is_match(sha))
                    {
                        issue(
                            &mut errors,
                            name,
                            "hook revision must be an immutable commit",
                        );
                    }
                    if let Some(items) = repo.get("hooks").and_then(Yaml::as_array) {
                        hooks.extend(
                            items
                                .iter()
                                .filter_map(|h| h.get("id").and_then(Yaml::as_str)),
                        );
                    }
                }
                for required in ["repo-policy", "rust-fmt", "rust-clippy", "rust-tests"] {
                    if !hooks.contains(&required) {
                        issue(
                            &mut errors,
                            name,
                            format!("missing required hook {required}"),
                        );
                    }
                }
            }
            Err(error) => issue(&mut errors, name, format!("invalid YAML: {error}")),
        }
    }
    if extension == "json"
        && let Err(error) = serde_json::from_str::<serde_json::Value>(text)
    {
        issue(&mut errors, name, format!("invalid JSON: {error}"));
    }
    errors
}

fn is_restricted(name: &str) -> bool {
    SENSITIVE.is_match(name)
        && !matches!(
            Path::new(name).file_name().and_then(|file| file.to_str()),
            Some(".env.example" | ".env.sample" | ".env.template")
        )
}

fn inspect_repository() -> Result<Vec<String>, String> {
    let result = Command::new("git")
        .args(["ls-files", "--cached", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err("git ls-files failed".into());
    }
    let names = std::str::from_utf8(&result.stdout).map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    for name in names.split('\0').filter(|n| !n.is_empty()) {
        if is_restricted(name) {
            issue(
                &mut errors,
                name,
                "private/generated material must not be tracked",
            );
            continue;
        }
        let path = Path::new(name);
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if name != "LICENSE.md"
            && !matches!(
                extension,
                "rs" | "js" | "mjs" | "ts" | "py" | "toml" | "json" | "yml" | "yaml" | "sh"
            )
        {
            continue;
        }
        let staged = Command::new("git")
            .args(["show", &format!(":{name}")])
            .output()
            .map_err(|e| e.to_string())?;
        if !staged.status.success() {
            return Err(format!("cannot inspect staged {name}"));
        }
        let text = std::str::from_utf8(&staged.stdout).map_err(|e| format!("{name}: {e}"))?;
        errors.extend(inspect_text(name, text));
    }
    Ok(errors)
}

fn ci_outcomes(text: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let jobs = value
        .as_object()
        .filter(|jobs| !jobs.is_empty())
        .ok_or("missing prerequisite outcomes")?;
    Ok(jobs
        .iter()
        .filter(|(_, outcome)| {
            outcome.get("result").and_then(serde_json::Value::as_str) != Some("success")
        })
        .map(|(job, _)| format!("required job {job} did not succeed"))
        .collect())
}

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        None => inspect_repository(),
        Some("ci-gate") => std::env::var("AARU_CI_NEEDS")
            .map_err(|error| error.to_string())
            .and_then(|text| ci_outcomes(&text)),
        Some(argument) => Err(format!("unknown policy command: {argument}")),
    };
    match result {
        Ok(errors) if errors.is_empty() => ExitCode::SUCCESS,
        Ok(errors) => {
            for error in errors {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("repository policy could not run: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ci_outcomes, inspect_text, is_restricted};

    #[test]
    fn aggregate_gate_rejects_skipped_failed_cancelled_and_missing_jobs() {
        assert!(
            ci_outcomes(r#"{"native":{"result":"success"},"security":{"result":"success"}}"#)
                .unwrap()
                .is_empty()
        );
        for status in ["skipped", "failure", "cancelled", "unknown"] {
            let outcomes = format!(
                r#"{{"native":{{"result":"success"}},"security":{{"result":"{status}"}}}}"#
            );
            assert_eq!(
                ci_outcomes(&outcomes).unwrap(),
                vec!["required job security did not succeed"]
            );
        }
        assert!(!ci_outcomes(r#"{"native":{}}"#).unwrap().is_empty());
        assert!(ci_outcomes("{}").is_err());
        assert!(ci_outcomes("[]").is_err());
        assert!(ci_outcomes("not json").is_err());
    }

    #[test]
    fn first_party_license_is_validated_independently_of_dependency_licenses() {
        let license = include_str!("../../../LICENSE.md");
        assert!(inspect_text("LICENSE.md", license).is_empty());
        assert!(
            !inspect_text("LICENSE.md", &license.replace("Strict", "Noncommercial")).is_empty()
        );
    }

    #[test]
    fn dependency_ranges_and_git_sources_are_rejected() {
        for version in [
            "\"1.2\"",
            "\"^1.2.3\"",
            "{ git = \"https://example.test/a\" }",
        ] {
            assert!(
                !inspect_text("Cargo.toml", &format!("[dependencies]\na = {version}\n")).is_empty()
            );
        }
        assert!(inspect_text("Cargo.toml", "[dependencies]\na = \"=1.2.3\"\n").is_empty());
        assert!(
            inspect_text(
                "Cargo.toml",
                "[workspace.dependencies]\na = \"=1.1.8+spec-1.1.0\"\n"
            )
            .is_empty()
        );
        assert!(
            inspect_text("Cargo.toml", "[dependencies]\na = { workspace = true }\n").is_empty()
        );
    }

    #[test]
    fn source_suppressions_and_conditional_disabled_tests_are_rejected() {
        let cases = [
            ("x.rs", ["#[", "allow(clippy::all)]"].concat()),
            ("x.rs", ["#[", "ignore]"].concat()),
            ("x.rs", ["#[", "cfg_attr(test, allow(dead_code))]"].concat()),
            ("x.rs", ["#[", "cfg_attr(test, ignore)]"].concat()),
            ("x.js", ["// ", "biome-ignore lint: reason"].concat()),
            (
                "x.js",
                ["alert('x'); // ", "eslint-disable-line no-alert"].concat(),
            ),
            ("x.js", ["test.", "skip('broken', () => {})"].concat()),
            ("x.py", ["import os  # ", "noqa: F401"].concat()),
        ];
        for (name, text) in cases {
            assert!(!inspect_text(name, &text).is_empty(), "accepted {text}");
        }
        assert!(inspect_text("x.rs", "#[derive(Debug)]\nstruct Valid;").is_empty());
    }

    #[test]
    fn lint_weakening_and_toolchain_drift_are_rejected() {
        for section in ["lints.rust", "workspace.lints.clippy"] {
            assert!(
                !inspect_text(
                    "Cargo.toml",
                    &format!("[{section}]\nunsafe_code = \"allow\"\n")
                )
                .is_empty()
            );
        }
        assert!(inspect_text("Cargo.toml", "[lints.rust]\nunsafe_code = \"forbid\"\n").is_empty());
        assert!(
            !inspect_text("rust-toolchain.toml", "[toolchain]\nchannel = \"stable\"").is_empty()
        );
        assert!(
            inspect_text("rust-toolchain.toml", "[toolchain]\nchannel = \"1.98.1\"").is_empty()
        );
    }

    fn hardened_workflow() -> String {
        let sha = "a".repeat(40);
        format!(
            "permissions:\n  contents: read\njobs:\n  native:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 20\n    steps:\n      - uses: step-security/harden-runner@{sha}\n        with:\n          egress-policy: block\n      - uses: actions/checkout@{sha}\n        with:\n          persist-credentials: false\n"
        )
    }

    #[test]
    fn each_job_requires_first_step_hardening_and_safe_checkout() {
        let safe = hardened_workflow();
        assert!(inspect_text(".github/workflows/example.yml", &safe).is_empty());
        for faulty in [
            safe.replace(&"a".repeat(40), "v4"),
            safe.replace("timeout-minutes: 20", "timeout-minutes: 0"),
            safe.replace("persist-credentials: false", "persist-credentials: true"),
            safe.replace("contents: read", "contents: write"),
            safe.replace("egress-policy: block", "egress-policy: audit"),
            safe.replace(
                "egress-policy: block",
                "egress-policy: block\n        if: false",
            ),
            format!(
                "{safe}  unprotected:\n    runs-on: windows-2025\n    timeout-minutes: 10\n    steps:\n      - run: cargo test --locked\n"
            ),
            format!("{safe}    continue-on-error: true\n"),
        ] {
            assert!(
                !inspect_text(".github/workflows/example.yml", &faulty).is_empty(),
                "accepted {faulty}"
            );
        }
    }

    #[test]
    fn aggregate_workflow_cannot_omit_prerequisites_or_fake_a_success() {
        let sha = "a".repeat(40);
        let safe = format!(
            "{}  gate:\n    name: Quality gate\n    needs: [native]\n    if: always()\n    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n    steps:\n      - uses: step-security/harden-runner@{sha}\n        with:\n          egress-policy: block\n      - run: cargo run --locked --quiet -p aaru-repo-policy -- ci-gate\n        env:\n          AARU_CI_NEEDS: ${{{{ toJSON(needs) }}}}\n",
            hardened_workflow()
        );
        assert!(inspect_text(".github/workflows/ci.yml", &safe).is_empty());
        for faulty in [
            safe.replace("needs: [native]", "needs: []"),
            safe.replace("if: always()", "if: success()"),
            safe.replace(
                "cargo run --locked --quiet -p aaru-repo-policy -- ci-gate",
                "echo success",
            ),
            safe.replace("toJSON(needs)", "toJSON(github)"),
        ] {
            assert!(
                !inspect_text(".github/workflows/ci.yml", &faulty).is_empty(),
                "accepted {faulty}"
            );
        }
    }

    #[test]
    fn restricted_paths_are_rejected_before_any_content_read() {
        for name in [
            "auth.json",
            "service-account-prod.json",
            "local-credentials.json",
            "vault.yaml",
            ".npmrc",
            ".env.local",
            ".env.dev.sample",
            "nested/credentials.json",
            "NODE_MODULES/x.js",
            "target/a.rs",
            "web/pkg/a.js",
        ] {
            assert!(is_restricted(name), "accepted {name}");
        }
        for name in [
            ".env.example",
            ".env.sample",
            "nested/.env.template",
            "fixtures/realistic.json",
            "crates/aaru-contract/src/lib.rs",
        ] {
            assert!(!is_restricted(name), "rejected {name}");
        }
    }

    #[test]
    fn missing_hooks_and_mutable_hook_revisions_are_rejected() {
        let sha = "a".repeat(40);
        let safe = format!(
            "repos:\n  - repo: https://example.test/hooks\n    rev: {sha}\n    hooks:\n      - id: general\n  - repo: local\n    hooks:\n      - id: repo-policy\n      - id: rust-fmt\n      - id: rust-clippy\n      - id: rust-tests\n"
        );
        assert!(inspect_text(".pre-commit-config.yaml", &safe).is_empty());
        assert!(!inspect_text(".pre-commit-config.yaml", &safe.replace(&sha, "v1")).is_empty());
        assert!(
            !inspect_text(
                ".pre-commit-config.yaml",
                &safe.replace("id: rust-tests", "id: other")
            )
            .is_empty()
        );
    }

    #[test]
    fn hook_bypass_is_rejected_without_requiring_a_literal_in_this_source() {
        for text in [
            ["git commit --no-", "verify"].concat(),
            ["S", "KIP=rust-tests git commit"].concat(),
            ["git -c core.hooks", "Path=/tmp commit"].concat(),
        ] {
            assert!(!inspect_text("script.sh", &text).is_empty());
        }
        assert!(inspect_text("script.sh", "cargo test --locked").is_empty());
    }
}
