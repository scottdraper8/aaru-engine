"""Independent fault cases for the repository's enforcement boundary."""

import unittest

from repo_policy import inspect_text


class PolicyTests(unittest.TestCase):
    def test_dependency_ranges_and_git_sources_are_rejected(self):
        for value in ('"1.2"', '"^1.2.3"', '{ git = "https://example.test/a" }'):
            with self.subTest(value=value):
                self.assertTrue(
                    inspect_text("Cargo.toml", f"[dependencies]\na = {value}\n")
                )
        self.assertEqual(
            inspect_text("Cargo.toml", '[dependencies]\na = "=1.2.3"\n'), []
        )
        self.assertTrue(
            inspect_text("package.json", '{"devDependencies":{"a":"~1.2.3"}}')
        )
        self.assertEqual(
            inspect_text("package.json", '{"dependencies":{"a":"1.2.3"}}'), []
        )

    def test_action_tags_and_missing_hardening_are_rejected(self):
        self.assertTrue(
            inspect_text(".github/workflows/ci.yml", "- uses: actions/checkout@v4")
        )
        sha = "a" * 40
        safe = (
            f"- uses: step-security/harden-runner@{sha}\n- uses: actions/checkout@{sha}"
        )
        self.assertEqual(inspect_text(".github/workflows/ci.yml", safe), [])

    def test_suppressions_and_disabled_tests_are_rejected(self):
        cases = [
            ("x.rs", "#[" + "allow(clippy::all)]"),
            ("x.rs", "#[" + "ignore]"),
            ("x.js", "// " + "biome-ignore lint: reason"),
            ("x.js", "test." + "skip('broken', () => {})"),
            ("x.py", "# " + "noqa"),
            ("x.py", "import os  # " + "noqa: F401"),
            ("x.js", "alert('x'); // " + "eslint-disable-line no-alert"),
            ("x.rs", "#[" + "cfg_attr(test, allow(dead_code))]"),
            ("x.rs", "#[" + "cfg_attr(test, ignore)]"),
        ]
        for name, text in cases:
            with self.subTest(name=name, text=text):
                self.assertTrue(inspect_text(name, text))
        self.assertEqual(inspect_text("x.rs", "#[derive(Debug)]\nstruct Valid;"), [])

    def test_cargo_lint_weakening_is_rejected(self):
        for section in ("lints.rust", "workspace.lints.clippy"):
            text = f'[{section}]\nunsafe_code = "' + 'allow"\n'
            with self.subTest(section=section):
                self.assertTrue(inspect_text("Cargo.toml", text))
        self.assertEqual(
            inspect_text("Cargo.toml", '[lints.rust]\nunsafe_code = "forbid"\n'), []
        )

    def test_toolchain_drift_is_rejected(self):
        self.assertTrue(
            inspect_text("rust-toolchain.toml", '[toolchain]\nchannel = "stable"')
        )
        self.assertEqual(
            inspect_text("rust-toolchain.toml", '[toolchain]\nchannel = "1.98.1"'), []
        )


if __name__ == "__main__":
    unittest.main()
