extern crate assert_cli;
#[macro_use]
extern crate pretty_assertions;
extern crate tempdir;
extern crate toml_edit;

use std::fs;

mod utils;
use utils::{clone_out_test, execute_command, get_toml};

// Verify that an upgraded Cargo.toml matches what we expect.
#[test]
fn upgrade_as_expected() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.source");

    execute_command(&["upgrade"], &manifest);

    let upgraded = get_toml(&manifest);
    let target = get_toml("tests/fixtures/upgrade/Cargo.toml.target");

    assert_eq!(target.to_string(), upgraded.to_string());
}

#[test]
fn upgrade_all() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependency `versioned-package@0.1.1`
    execute_command(&["add", "versioned-package", "--vers", "0.1.1"], &manifest);

    // Now, upgrade `versioned-package` to the latest version
    execute_command(&["upgrade"], &manifest);

    // Verify that `versioned-package` has been updated successfully.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["versioned-package"].as_str(),
        Some("versioned-package--CURRENT_VERSION_TEST")
    );
}

#[test]
fn upgrade_all_allow_prerelease() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependency `versioned-package@0.1.1`
    execute_command(&["add", "versioned-package", "--vers", "0.1.1"], &manifest);

    // Now, upgrade `versioned-package` to the latest version
    execute_command(&["upgrade", "--allow-prerelease"], &manifest);

    // Verify that `versioned-package` has been updated successfully.
    assert_eq!(
        get_toml(&manifest)["dependencies"]["versioned-package"].as_str(),
        Some("versioned-package--PRERELEASE_VERSION_TEST")
    );
}

#[test]
fn upgrade_specified_only() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Setup manifest with the dependencies `versioned-package` and `versioned-package-2`
    execute_command(&["add", "versioned-package", "--vers", "0.1.1"], &manifest);
    execute_command(
        &["add", "versioned-package-2", "--vers", "0.1.1"],
        &manifest,
    );

    // Update `versioned-package` to the latest version
    execute_command(&["upgrade", "-d", "versioned-package"], &manifest);

    // Verify that `versioned-package` was upgraded, but not `versioned-package-2`
    let dependencies = &get_toml(&manifest)["dependencies"];
    assert_eq!(
        dependencies["versioned-package"].as_str(),
        Some("versioned-package--CURRENT_VERSION_TEST")
    );
    assert_eq!(
        dependencies["versioned-package-2"].as_str(),
        Some("0.1.1")
    );
}

#[test]
fn fails_to_upgrade_missing_dependency() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");

    // Update the non-existent `failure` to the latest version
    execute_command(&["upgrade", "-d", "failure"], &manifest);

    // Verify that `failure` has not been added
    assert!(get_toml(&manifest)["dependencies"]["failure"].is_none());
}

#[test]
fn upgrade_optional_dependency() {
    // Set up a Cargo.toml with an optional dependency `test_optional_dependency` verifies that this
    // is correct.
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/add/Cargo.toml.sample");
    execute_command(
        &[
            "add",
            "versioned-package",
            "--vers",
            ">=0.1.1",
            "--optional",
        ],
        &manifest,
    );

    // Now, update without including the `optional` flag.
    execute_command(&["upgrade"], &manifest);

    // Dependency present afterwards - correct version, and still optional.
    let toml = get_toml(&manifest);
    let val = &toml["dependencies"]["versioned-package"];
    assert_eq!(
        val["version"].as_str(),
        Some("versioned-package--CURRENT_VERSION_TEST")
    );
    assert_eq!(val["optional"].as_bool(), Some(true));
}

#[test]
fn upgrade_workspace() {
    // Create a temporary directory and copy in the root manifest, the dummy rust file, and
    // workspace member manifests.
    let tmpdir = tempdir::TempDir::new("upgrade_workspace")
        .expect("failed to construct temporary directory");

    // Helper to copy in files to the temporary workspace. The standard library doesn't have a good
    // equivalent of `cp -r`, hence this oddity.
    let copy_in = |dir, file| {
        let file_path = tmpdir
            .path()
            .join(dir)
            .join(file)
            .to_str()
            .unwrap()
            .to_string();

        fs::create_dir_all(tmpdir.path().join(dir)).unwrap();

        fs::copy(
            format!("tests/fixtures/workspace/{}/{}", dir, file),
            &file_path,
        ).unwrap_or_else(|err| panic!("could not copy test file: {}", err));

        file_path
    };

    let root_manifest = copy_in(".", "Cargo.toml");
    copy_in(".", "dummy.rs");

    let workspace_manifests = &["one", "two", "implicit/three", "explicit/four"]
        .iter()
        .map(|member| copy_in(member, "Cargo.toml"))
        .collect::<Vec<_>>();

    execute_command(&["upgrade", "--all"], &root_manifest);

    // All of the workspace members have `libc` as a dependency.
    for workspace_member in workspace_manifests {
        assert_eq!(
            get_toml(workspace_member)["dependencies"]["libc"].as_str(),
            Some("libc--CURRENT_VERSION_TEST")
        );
    }
}

/// Detect if attempting to run against a workspace root and give a helpful warning.
#[test]
fn detect_workspace() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/workspace/Cargo.toml");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "--manifest-path",
        &manifest,
    ]).fails_with(1)
        .prints_error_exactly(
            "Command failed due to unhandled error: Failed to write new manifest contents

Caused by: Found virtual manifest, but this command requires running against an actual package in \
this workspace.",
        )
        .unwrap();
}

#[test]
fn invalid_manifest() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.invalid");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "--manifest-path",
        &manifest,
    ]).fails_with(1)
        .prints_error_exactly(
            "Command failed due to unhandled error: Unable to parse Cargo.toml

Caused by: Manifest not valid TOML
Caused by: TOML parse error at line 1, column 6
  |
1 | This is clearly not a valid Cargo.toml.
  |      ^
Unexpected `i`
Expected `=`",
        )
        .unwrap();
}

#[test]
fn invalid_root_manifest() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.invalid");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "--all",
        "--manifest-path",
        &manifest,
    ]).fails_with(1)
        .prints_error("Command failed due to unhandled error: Failed to get metadata")
        .unwrap();
}

#[test]
fn unknown_flags() {
    assert_cli::Assert::command(&["target/debug/cargo-upgrade", "upgrade", "foo", "--flag"])
        .fails_with(1)
        .prints_error_exactly(
            "Unknown flag: '--flag'

Usage:
    cargo upgrade [--all] [--dependency <dep>...] [--manifest-path <path>] [options]
    cargo upgrade (-h | --help)
    cargo upgrade (-V | --version)",
        )
        .unwrap();
}

#[test]
fn upgrade_prints_messages() {
    let (_tmpdir, manifest) = clone_out_test("tests/fixtures/upgrade/Cargo.toml.source");

    assert_cli::Assert::command(&[
        "target/debug/cargo-upgrade",
        "upgrade",
        "-d",
        "docopt",
        &format!("--manifest-path={}", manifest),
    ]).succeeds()
        .prints("docopt v0.8 -> v")
        .unwrap();
}
