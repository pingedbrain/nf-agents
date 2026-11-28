# SPDX-License-Identifier: Apache-2.0
"""Tests for scripts/release/check.sh: run it against temp workspaces and against the repo.

Run: python3 -I -m unittest discover -s scripts/release -p 'test_*.py'
"""
import os
import subprocess
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
CHECK = os.path.join(HERE, "check.sh")

FIELDS = {
    "description": "Test crate.",
    "license": "Apache-2.0",
    "repository": "https://example.com/repo",
    "readme": "README.md",
    "keywords": ["test"],
    "categories": ["development-tools"],
}


def toml(value):
    if isinstance(value, list):
        return "[" + ", ".join(f'"{item}"' for item in value) + "]"
    return f'"{value}"'


def manifest(name, version, drop=(), extra=()):
    lines = ["[package]", f'name = "{name}"', f'version = "{version}"', 'edition = "2024"']
    lines += [f"{key} = {toml(value)}" for key, value in FIELDS.items() if key not in drop]
    lines += list(extra)
    return "\n".join(lines) + "\n"


def make_workspace(root, crates):
    """crates maps crate name to its Cargo.toml text."""
    members = ", ".join(f'"crates/{name}"' for name in crates)
    write(os.path.join(root, "Cargo.toml"), f'[workspace]\nresolver = "3"\nmembers = [{members}]\n')
    for name, text in crates.items():
        crate = os.path.join(root, "crates", name)
        write(os.path.join(crate, "Cargo.toml"), text)
        write(os.path.join(crate, "src", "lib.rs"), "")


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def run_check(cwd, *args):
    return subprocess.run([CHECK, *args], cwd=cwd, capture_output=True, text=True, check=False)


class CheckScriptTests(unittest.TestCase):
    def test_repo_root_passes(self):
        result = run_check(REPO)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_matching_prerelease_tag_passes(self):
        with tempfile.TemporaryDirectory() as tmp:
            make_workspace(tmp, {
                "alpha": manifest("alpha", "1.2.3-rc.1"),
                "beta": manifest("beta", "1.2.3-rc.1"),
            })
            result = run_check(tmp, "v1.2.3-rc.1")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_tag_without_prerelease_suffix_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            make_workspace(tmp, {
                "alpha": manifest("alpha", "1.2.3-rc.1"),
                "beta": manifest("beta", "1.2.3-rc.1"),
            })
            result = run_check(tmp, "v1.2.3")
        self.assertEqual(result.returncode, 1)

    def test_versions_differ_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            make_workspace(tmp, {
                "alpha": manifest("alpha", "1.2.3"),
                "beta": manifest("beta", "1.2.4"),
            })
            result = run_check(tmp)
        self.assertEqual(result.returncode, 1)
        self.assertIn("versions differ", result.stderr)

    def assert_missing_field_fails(self, field):
        with tempfile.TemporaryDirectory() as tmp:
            make_workspace(tmp, {
                "alpha": manifest("alpha", "1.2.3"),
                "beta": manifest("beta", "1.2.3", drop=(field,)),
            })
            result = run_check(tmp)
        self.assertEqual(result.returncode, 1)
        self.assertIn("beta", result.stderr)

    def test_missing_readme_names_crate(self):
        self.assert_missing_field_fails("readme")

    def test_missing_keywords_names_crate(self):
        self.assert_missing_field_fails("keywords")

    def test_missing_categories_names_crate(self):
        self.assert_missing_field_fails("categories")

    def test_publish_false_names_crate(self):
        with tempfile.TemporaryDirectory() as tmp:
            make_workspace(tmp, {
                "alpha": manifest("alpha", "1.2.3"),
                "beta": manifest("beta", "1.2.3", extra=["publish = false"]),
            })
            result = run_check(tmp)
        self.assertEqual(result.returncode, 1)
        self.assertIn("beta", result.stderr)

    def test_two_arguments_exit_2(self):
        result = run_check(REPO, "v1", "v2")
        self.assertEqual(result.returncode, 2)


if __name__ == "__main__":
    unittest.main()
