"""Fail closed on repository-specific supply-chain and check-bypass risks."""

import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EXACT = re.compile(r"^=?\d+\.\d+\.\d+(?:-[\w.-]+)?$")
SHA = re.compile(r"^[0-9a-f]{40}$")
SUPPRESSION = re.compile(
    r"(?m)(?://|#|/\*|<!--)\s*(?:"
    r"(?:eslint|biome|stylelint)[- ](?:disable|ignore)|"
    r"@ts-(?:ignore|nocheck)|noqa\b|nosec\b|type:\s*ignore|"
    r"pre-commit:\s*skip)"
    r"|#\s*\[\s*(?:allow|expect)\s*\("
    r"|#\s*\[\s*ignore\b"
    r"|#\s*\[\s*cfg_attr\b[^\]]*\b(?:allow|expect|ignore)\b"
    r"|\b(?:test|describe|it)\.(?:skip|only|todo)\s*\("
)
SENSITIVE = re.compile(
    r"(?:^|/)(?:\.env(?:\..*)?|auth\.json|credentials\.json|"
    r"secrets\.(?:json|ya?ml|toml)|id_(?:rsa|ed25519|ecdsa).*|"
    r"private-data|node_modules|target)(?:/|$)|\.(?:pem|key|p12|pfx)$"
)


def inspect_text(name: str, text: str) -> list[str]:
    """Inspect source/config text; documentation examples are not executable."""
    errors = []
    path = Path(name)
    if path.suffix in {".rs", ".js", ".mjs", ".ts", ".py"} and SUPPRESSION.search(text):
        errors.append(f"{name}: inline suppression or disabled/focused test")
    if name.startswith(".github/workflows/"):
        for action in re.findall(r"\buses:\s*[\"']?([^\s\"']+)", text):
            if action.startswith("./"):
                continue
            if "@" not in action or not SHA.fullmatch(action.rsplit("@", 1)[1]):
                errors.append(
                    f"{name}: action must use a full immutable commit: {action}"
                )
        if "step-security/harden-runner@" not in text:
            errors.append(f"{name}: hosted workflow must harden its runners")
        if re.search(
            r"pull_request_target|secrets:\s*inherit|persist-credentials:\s*true", text
        ):
            errors.append(f"{name}: unsafe privilege boundary")
    if path.name == "Cargo.toml":
        data = tomllib.loads(text)

        def dependencies(table):
            for key, value in table.items():
                if key.endswith("dependencies") and isinstance(value, dict):
                    for dep, spec in value.items():
                        if isinstance(spec, dict) and (
                            "path" in spec or spec.get("workspace")
                        ):
                            continue
                        version = (
                            spec if isinstance(spec, str) else spec.get("version", "")
                        )
                        if not version.startswith("=") or not EXACT.fullmatch(version):
                            errors.append(
                                f"{name}: dependency {dep} needs an exact =version"
                            )
                elif isinstance(value, dict):
                    dependencies(value)

        dependencies(data)

        def inspect_lints(table):
            for key, value in table.items():
                if value == "allow" or (
                    isinstance(value, dict) and value.get("level") == "allow"
                ):
                    errors.append(f"{name}: lint {key} cannot be weakened to allow")
                elif isinstance(value, dict):
                    inspect_lints(value)

        inspect_lints(data.get("lints", {}))
        inspect_lints(data.get("workspace", {}).get("lints", {}))
    if path.name == "package.json":
        data = json.loads(text)
        for section in ("dependencies", "devDependencies", "optionalDependencies"):
            for dep, version in data.get(section, {}).items():
                if not EXACT.fullmatch(version) or version.startswith("="):
                    errors.append(f"{name}: dependency {dep} needs an exact version")
    if name == "rust-toolchain.toml":
        channel = tomllib.loads(text)["toolchain"]["channel"]
        if not EXACT.fullmatch(channel):
            errors.append(f"{name}: Rust toolchain must use an exact version")
    if name == ".pre-commit-config.yaml":
        for revision in re.findall(r"\brev:\s*(\S+)", text):
            if not SHA.fullmatch(revision):
                errors.append(f"{name}: hook revision must be an immutable commit")
        for hook in (
            "repo-policy",
            "policy-tests",
            "rust-fmt",
            "rust-clippy",
            "rust-tests",
            "web-check",
        ):
            if f"id: {hook}" not in text:
                errors.append(f"{name}: missing required hook {hook}")
    if path.suffix in {
        ".yml",
        ".yaml",
        ".sh",
        ".py",
        ".js",
        ".mjs",
        ".json",
    } and re.search(r"--no-" r"verify\b|\bSKIP\s*=|core\.hooksPath\s*[= ]", text):
        errors.append(f"{name}: executable hook bypass")
    return errors


def main() -> int:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "-z"], cwd=ROOT, check=True, capture_output=True
    )
    errors = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        name = raw.decode("utf-8")
        if SENSITIVE.search(name) and not name.endswith(
            (".example", ".sample", ".template")
        ):
            errors.append(f"{name}: private/generated material must not be tracked")
            continue
        path = ROOT / name
        if (
            path.suffix
            in {
                ".rs",
                ".js",
                ".mjs",
                ".ts",
                ".py",
                ".toml",
                ".json",
                ".yml",
                ".yaml",
                ".sh",
            }
            and path.is_file()
        ):
            errors.extend(inspect_text(name, path.read_text(encoding="utf-8")))
    for error in errors:
        print(error, file=sys.stderr)
    return bool(errors)


if __name__ == "__main__":
    sys.exit(main())
