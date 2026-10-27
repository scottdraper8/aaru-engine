use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Repository(PathBuf);

impl Repository {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "aaru-policy-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let repository = Self(root);
        repository.git(&["init", "--quiet"]);
        repository
    }

    fn git(&self, args: &[&str]) -> Output {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn write(&self, name: &str, bytes: impl AsRef<[u8]>) {
        fs::write(self.0.join(name), bytes).unwrap();
    }

    fn policy(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_aaru-repo-policy"))
            .current_dir(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cli_reads_the_staged_blob_in_both_directions_not_the_worktree() {
    let repository = Repository::new();
    let exact = "[dependencies]\nexample = \"=1.2.3\"\n";
    let range = "[dependencies]\nexample = \"1.2\"\n";
    repository.write("Cargo.toml", exact);
    repository.git(&["add", "Cargo.toml"]);
    repository.write("Cargo.toml", range);
    assert!(repository.policy().status.success());
    repository.git(&["add", "Cargo.toml"]);
    repository.write("Cargo.toml", exact);
    let rejected = repository.policy();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("needs an exact"));
}

#[test]
fn cli_checks_document_hygiene_and_all_first_party_source_extensions() {
    let repository = Repository::new();
    repository.write("notes.md", "Trailing space \n");
    repository.git(&["add", "notes.md"]);
    let rejected = repository.policy();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("trailing whitespace"));
    repository.write("notes.md", "Clean text.\n");
    repository.git(&["add", "notes.md"]);
    assert!(repository.policy().status.success());
    repository.write("ui.tsx", "export const value = 1;\n");
    repository.git(&["add", "ui.tsx"]);
    let rejected = repository.policy();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("must use Rust"));
}

#[test]
fn raw_cases_keep_invalid_input_bytes_but_cannot_hide_key_material() {
    let repository = Repository::new();
    repository.write("input.case", [0xff, 0, b' ', b'\r', b'\n']);
    repository.git(&["add", "input.case"]);
    assert!(repository.policy().status.success());
    repository.write(
        "input.case",
        format!("-----BEGIN {} PRIVATE KEY-----\n", "RSA"),
    );
    repository.git(&["add", "input.case"]);
    let rejected = repository.policy();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("private-key material"));
}

#[test]
fn cli_enforces_size_before_reading_content_and_rejects_symlink_modes() {
    let repository = Repository::new();
    let asset = fs::File::create(repository.0.join("asset.bin")).unwrap();
    asset.set_len(2_097_152).unwrap();
    repository.git(&["add", "asset.bin"]);
    assert!(repository.policy().status.success());
    asset.set_len(2_097_153).unwrap();
    repository.git(&["add", "asset.bin"]);
    let rejected = repository.policy();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("2048 KiB"));
    repository.git(&["rm", "--cached", "asset.bin"]);
    repository.write("target.txt", "absent.rs");
    let hash = repository.git(&["hash-object", "-w", "target.txt"]);
    let oid = std::str::from_utf8(&hash.stdout).unwrap().trim();
    repository.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("120000,{oid},link.rs"),
    ]);
    let rejected = repository.policy();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("symlinks"));
}
