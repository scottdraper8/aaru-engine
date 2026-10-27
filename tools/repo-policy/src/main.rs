mod framework_security;
mod hygiene;

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
        r"\.netrc|\.npmrc|\.pypirc|\.DS_Store|Thumbs\.db|Desktop\.ini|private-data|node_modules|",
        r"bower_components|vendor|\.npm|\.yarn|\.pnp\.[^/]*|",
        r"\.venv|__pycache__|dist|build|out|\.idea|\.vscode|target|web/pkg|\.site)",
        r"(?:/|$)|\.(?:good\.json|aaru\.json|pem|key|p12|pfx|pyc|o|so|dylib|swp|swo)$|~$"
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

fn runs_unconditionally(job: &Yaml, command: &str) -> bool {
    job.get("steps")
        .and_then(Yaml::as_array)
        .is_some_and(|steps| {
            steps.iter().any(|step| {
                step.get("if").is_none()
                    && step.get("run").and_then(Yaml::as_str).map(str::trim) == Some(command)
            })
        })
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
        for (id, commands) in [
            (
                "native",
                &[
                    "python -m pip install --require-hashes --only-binary=:all: -r .github/requirements-precommit.txt",
                    "python -m pre_commit run --all-files --show-diff-on-failure",
                ][..],
            ),
            (
                "supply-chain",
                &[
                    "cargo audit --deny warnings",
                    "cargo deny --locked check",
                    "cargo run --locked --quiet -p aaru-repo-policy -- framework-advisories",
                ][..],
            ),
        ] {
            if let Some(job) = jobs.get(id) {
                for command in commands {
                    if !runs_unconditionally(job, command) {
                        issue(errors, name, format!("{id} must always run {command}"));
                    }
                }
            } else {
                issue(errors, name, format!("missing required {id} job"));
            }
        }
        let native = jobs.get("native");
        if !native
            .and_then(|job| job.get("steps"))
            .and_then(Yaml::as_array)
            .is_some_and(|steps| {
                steps.iter().any(|step| {
                    step.get("if").is_none()
                        && step
                            .get("uses")
                            .and_then(Yaml::as_str)
                            .is_some_and(|action| action.starts_with("actions/setup-python@"))
                })
            })
        {
            issue(
                errors,
                name,
                "native must install an exactly pinned Python runtime",
            );
        }
        let matrix = native
            .and_then(|job| job.get("strategy"))
            .and_then(|strategy| strategy.get("matrix"));
        let platforms = matrix
            .and_then(|matrix| matrix.get("os"))
            .and_then(Yaml::as_array);
        let bound_platforms = native
            .and_then(|job| job.get("runs-on"))
            .and_then(Yaml::as_str)
            == Some("${{ matrix.os }}")
            && matrix.is_some_and(|matrix| matrix.get("exclude").is_none())
            && platforms.is_some_and(|platforms| {
                ["ubuntu-", "windows-"].into_iter().all(|prefix| {
                    platforms
                        .iter()
                        .any(|platform| platform.as_str().is_some_and(|s| s.starts_with(prefix)))
                })
            });
        if !bound_platforms {
            issue(
                errors,
                name,
                "native matrix must execute both Linux and Windows",
            );
        }
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
        if job
            .get("continue-on-error")
            .is_some_and(|value| value.as_bool() != Some(false))
        {
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
            if step
                .get("continue-on-error")
                .is_some_and(|value| value.as_bool() != Some(false))
            {
                issue(errors, name, format!("{label} cannot ignore step failures"));
            }
            if let Some(action) = step.get("uses").and_then(Yaml::as_str) {
                let version_key = if action.starts_with("actions/setup-python@") {
                    Some("python-version")
                } else if action.starts_with("dtolnay/rust-toolchain@") {
                    Some("toolchain")
                } else {
                    None
                };
                if let Some(key) = version_key
                    && !step
                        .get("with")
                        .and_then(|options| options.get(key))
                        .and_then(Yaml::as_str)
                        .is_some_and(|version| EXACT.is_match(version))
                {
                    issue(
                        errors,
                        name,
                        format!("{label}: {key} needs an exact version"),
                    );
                }
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
    if matches!(
        extension,
        "py" | "js" | "mjs" | "ts" | "tsx" | "jsx" | "sh" | "bash" | "ps1" | "bat" | "cmd"
    ) || path.file_name().and_then(|s| s.to_str()) == Some("package.json")
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
                    if repo.get("repo").and_then(Yaml::as_str) != Some("local") {
                        issue(
                            &mut errors,
                            name,
                            "hooks must use locked local Rust checks, not independently resolved environments",
                        );
                    }
                    if let Some(items) = repo.get("hooks").and_then(Yaml::as_array) {
                        hooks.extend(items);
                    }
                }
                for (required, entry) in [
                    (
                        "repo-policy",
                        "cargo run --locked --quiet -p aaru-repo-policy",
                    ),
                    ("rust-fmt", "cargo fmt --all --check"),
                    (
                        "rust-clippy",
                        "cargo clippy --workspace --all-targets --locked -- -D warnings",
                    ),
                    ("rust-tests", "cargo test --workspace --locked"),
                ] {
                    let matching: Vec<_> = hooks
                        .iter()
                        .filter(|hook| hook.get("id").and_then(Yaml::as_str) == Some(required))
                        .collect();
                    if matching.len() != 1 {
                        issue(
                            &mut errors,
                            name,
                            format!("need exactly one required hook {required}"),
                        );
                        continue;
                    }
                    let hook = matching[0];
                    let stages = hook
                        .get("stages")
                        .or_else(|| document.get("default_stages"));
                    let stage_enabled = stages.is_none_or(|stages| {
                        stages.as_array().is_some_and(|items| {
                            items
                                .iter()
                                .any(|stage| stage.as_str() == Some("pre-commit"))
                        })
                    });
                    if hook.get("entry").and_then(Yaml::as_str) != Some(entry)
                        || hook.get("language").and_then(Yaml::as_str) != Some("system")
                        || hook.get("always_run").and_then(Yaml::as_bool) != Some(true)
                        || hook.get("pass_filenames").and_then(Yaml::as_bool) != Some(false)
                        || hook.get("args").is_some_and(|args| {
                            args.as_array().is_none_or(|items| !items.is_empty())
                        })
                        || !stage_enabled
                    {
                        issue(
                            &mut errors,
                            name,
                            format!(
                                "{required} must run its exact full check unconditionally at pre-commit"
                            ),
                        );
                    }
                }
                if document
                    .get("default_install_hook_types")
                    .is_some_and(|types| {
                        types.as_array().is_none_or(|items| {
                            !items.iter().any(|kind| kind.as_str() == Some("pre-commit"))
                        })
                    })
                {
                    issue(
                        &mut errors,
                        name,
                        "hook installation must include pre-commit",
                    );
                }
            }
            Err(error) => issue(&mut errors, name, format!("invalid YAML: {error}")),
        }
    }
    errors
}

fn is_restricted(name: &str) -> bool {
    let path = Path::new(name);
    if matches!(
        path.file_name().and_then(|file| file.to_str()),
        Some(".env.example" | ".env.sample" | ".env.template")
    ) {
        return path
            .parent()
            .and_then(Path::to_str)
            .is_some_and(|parent| SENSITIVE.is_match(parent));
    }
    SENSITIVE.is_match(name)
}

fn inspect_repository() -> Result<Vec<String>, String> {
    let result = Command::new("git")
        .args(["ls-files", "--stage", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err("git ls-files failed".into());
    }
    let index = std::str::from_utf8(&result.stdout).map_err(|e| e.to_string())?;
    let mut errors = Vec::new();
    let mut names = Vec::new();
    for record in index.split('\0').filter(|record| !record.is_empty()) {
        let (header, name) = record.split_once('\t').ok_or("malformed index record")?;
        names.push(name);
        if is_restricted(name) {
            issue(
                &mut errors,
                name,
                "private/generated material must not be tracked",
            );
            continue;
        }
        if matches!(
            name,
            ".agent-docs/genshin_decision_engine_design_memo.md"
                | ".agent-docs/behavioral_testing_overhaul_agent_protocol.md"
                | ".agent-docs/Evidence_Governed_Agentic_Reverse_Engineering.pdf"
        ) {
            continue;
        }
        let fields: Vec<_> = header.split(' ').collect();
        if fields.len() != 3 || fields[2] != "0" {
            issue(&mut errors, name, "unmerged or malformed index entry");
            continue;
        }
        let oid = fields[1];
        if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("invalid index object ID".into());
        }
        let size_result = Command::new("git")
            .args(["cat-file", "-s", oid])
            .output()
            .map_err(|error| error.to_string())?;
        if !size_result.status.success() {
            return Err(format!("cannot inspect staged metadata for {name}"));
        }
        let size = std::str::from_utf8(&size_result.stdout)
            .map_err(|error| error.to_string())?
            .trim()
            .parse::<u64>()
            .map_err(|error| error.to_string())?;
        let metadata_errors = hygiene::metadata(name, fields[0], size);
        if !metadata_errors.is_empty() {
            errors.extend(metadata_errors);
            continue;
        }
        if !hygiene::is_text(name) {
            continue;
        }
        let staged = Command::new("git")
            .args(["cat-file", "blob", oid])
            .output()
            .map_err(|e| e.to_string())?;
        if !staged.status.success() {
            return Err(format!("cannot inspect staged {name}"));
        }
        if name.ends_with(".case") {
            errors.extend(hygiene::inspect_secrets(
                name,
                &String::from_utf8_lossy(&staged.stdout),
            ));
            continue;
        }
        let text = std::str::from_utf8(&staged.stdout).map_err(|e| format!("{name}: {e}"))?;
        errors.extend(hygiene::inspect(name, text));
        errors.extend(inspect_text(name, text));
    }
    errors.extend(hygiene::portable_paths(names));
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
        Some("framework-advisories") => {
            framework_security::audit(Path::new(".github/requirements-precommit.txt"))
        }
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
    use super::{Yaml, ci_outcomes, inspect_text, is_restricted};

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
            format!("{safe}    continue-on-error: ${{{{ always() }}}}\n"),
        ] {
            assert!(
                !inspect_text(".github/workflows/example.yml", &faulty).is_empty(),
                "accepted {faulty}"
            );
        }
    }

    #[test]
    fn aggregate_workflow_cannot_omit_prerequisites_or_fake_a_success() {
        let safe = include_str!("../../../.github/workflows/ci.yml");
        assert!(inspect_text(".github/workflows/ci.yml", safe).is_empty());
        for faulty in [
            safe.replace("needs: [native, supply-chain]", "needs: []"),
            safe.replace("if: ${{ always() }}", "if: ${{ success() }}"),
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
    fn required_ci_jobs_and_commands_cannot_disappear_behind_a_green_gate() {
        let safe: Yaml =
            serde_saphyr::from_str(include_str!("../../../.github/workflows/ci.yml")).unwrap();
        let mut removed_job = safe.clone();
        removed_job["jobs"]
            .as_object_mut()
            .unwrap()
            .remove("supply-chain");
        removed_job["jobs"]["quality"]["needs"] = serde_json::json!(["native"]);
        let errors = inspect_text(".github/workflows/ci.yml", &removed_job.to_string());
        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing required supply-chain job"))
        );
        let mut missing_python = safe.clone();
        missing_python["jobs"]["native"]["steps"]
            .as_array_mut()
            .unwrap()
            .retain(|step| {
                !step
                    .get("uses")
                    .and_then(Yaml::as_str)
                    .is_some_and(|action| action.starts_with("actions/setup-python@"))
            });
        let errors = inspect_text(".github/workflows/ci.yml", &missing_python.to_string());
        assert!(errors.iter().any(|error| error.contains("native must install an exactly pinned Python runtime")));
        for (job, command) in [
            (
                "native",
                "python -m pip install --require-hashes --only-binary=:all: -r .github/requirements-precommit.txt",
            ),
            (
                "native",
                "python -m pre_commit run --all-files --show-diff-on-failure",
            ),
            ("supply-chain", "cargo audit --deny warnings"),
            ("supply-chain", "cargo deny --locked check"),
            (
                "supply-chain",
                "cargo run --locked --quiet -p aaru-repo-policy -- framework-advisories",
            ),
        ] {
            let mut missing = safe.clone();
            let steps = missing["jobs"][job]["steps"].as_array_mut().unwrap();
            let position = steps
                .iter()
                .position(|step| step.get("run").and_then(Yaml::as_str) == Some(command))
                .unwrap();
            let mut conditional = missing.clone();
            conditional["jobs"][job]["steps"][position]["if"] = Yaml::Bool(false);
            missing["jobs"][job]["steps"]
                .as_array_mut()
                .unwrap()
                .remove(position);
            for faulty in [missing, conditional] {
                let errors = inspect_text(".github/workflows/ci.yml", &faulty.to_string());
                assert!(
                    errors
                        .iter()
                        .any(|error| error.contains(&format!("{job} must always run {command}")))
                );
            }
        }
        let mut linux_only = safe.clone();
        linux_only["jobs"]["native"]["strategy"]["matrix"]["os"] =
            serde_json::json!(["ubuntu-24.04"]);
        let mut excluded_windows = safe.clone();
        excluded_windows["jobs"]["native"]["strategy"]["matrix"]["exclude"] =
            serde_json::json!([{"os":"windows-2025"}]);
        let mut unbound = safe;
        unbound["jobs"]["native"]["runs-on"] = Yaml::String("ubuntu-24.04".into());
        unbound["jobs"]["native"]["steps"][0]["with"]["egress-policy"] =
            Yaml::String("block".into());
        for faulty in [linux_only, excluded_windows, unbound] {
            let errors = inspect_text(".github/workflows/ci.yml", &faulty.to_string());
            assert!(
                errors.iter().any(
                    |error| error.contains("native matrix must execute both Linux and Windows")
                )
            );
        }
    }

    #[test]
    fn workflow_tool_installations_require_exact_versions() {
        let safe = hardened_workflow();
        for (action, key, version) in [
            ("actions/setup-python", "python-version", "3.14.8"),
            ("dtolnay/rust-toolchain", "toolchain", "1.98.1"),
        ] {
            let pinned = format!(
                "{safe}      - uses: {action}@{}\n        with:\n          {key}: '{version}'\n",
                "a".repeat(40)
            );
            assert!(inspect_text(".github/workflows/example.yml", &pinned).is_empty());
            for drift in ["stable", "3.14", "${{ matrix.version }}"] {
                let errors = inspect_text(
                    ".github/workflows/example.yml",
                    &pinned.replace(version, drift),
                );
                assert!(
                    errors
                        .iter()
                        .any(|error| error.contains(&format!("{key} needs an exact version")))
                );
            }
        }
    }

    #[test]
    fn restricted_paths_are_rejected_before_any_content_read() {
        for name in [
            "auth.json",
            "service-account-prod.json",
            "local-credentials.json",
            "vault.yaml",
            "account.good.json",
            "account.aaru.json",
            ".DS_Store",
            "source.rs.swp",
            ".npmrc",
            ".env.local",
            ".env.dev.sample",
            "nested/credentials.json",
            "NODE_MODULES/x.js",
            "target/a.rs",
            "web/pkg/a.js",
            "private-data/.env.example",
            "node_modules/.env.sample",
            ".npm/cache/record.json",
            ".yarn/cache/package.zip",
            ".pnp.cjs",
            "bower_components/library/source.json",
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
    fn required_hooks_cannot_be_replaced_disabled_or_independently_resolved() {
        let safe = include_str!("../../../.pre-commit-config.yaml");
        assert!(inspect_text(".pre-commit-config.yaml", safe).is_empty());
        for faulty in [
            safe.replace("id: rust-tests", "id: other"),
            safe.replace("cargo test --workspace --locked", "echo success"),
            safe.replace("always_run: true", "always_run: false"),
            safe.replace("pass_filenames: false", "pass_filenames: true"),
            safe.replace("language: system", "language: python"),
            safe.replace(
                "id: rust-tests",
                "id: rust-tests\n        args: ['--', '--ignored']",
            ),
            format!("default_stages: [manual]\n{safe}"),
            format!("default_install_hook_types: [pre-push]\n{safe}"),
            safe.replace(
                "repo: local",
                &format!(
                    "repo: https://example.test/hooks\n    rev: {}",
                    "a".repeat(40)
                ),
            ),
        ] {
            assert!(
                !inspect_text(".pre-commit-config.yaml", &faulty).is_empty(),
                "accepted {faulty}"
            );
        }
    }

    #[test]
    fn hook_bypass_is_rejected_without_requiring_a_literal_in_this_source() {
        for text in [
            ["git commit --no-", "verify"].concat(),
            ["S", "KIP=rust-tests git commit"].concat(),
            ["git -c core.hooks", "Path=/tmp commit"].concat(),
        ] {
            assert!(!inspect_text("workflow.yml", &format!("run: {text}\n")).is_empty());
        }
        assert!(inspect_text("workflow.yml", "run: cargo test --locked\n").is_empty());
    }
}
