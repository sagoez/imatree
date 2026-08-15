//! End-to-end tests driving the installed binary.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn imatree(directory: &TempDir) -> Command {
    let mut command = Command::cargo_bin("imatree").expect("binary should build");
    command.args(["--path", directory.path().to_str().unwrap()]);
    command
}

#[test]
fn writes_a_png_named_after_the_caption() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello There"])
        .assert()
        .success();

    assert!(directory.path().join("hello_there.png").is_file());
}

#[test]
fn reports_where_the_image_was_written() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello"])
        .assert()
        .success()
        .stdout(predicate::str::contains("hello.png"));
}

#[test]
fn quiet_suppresses_the_success_message() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello", "--quiet"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn output_overrides_the_derived_file_name() {
    let directory = TempDir::new().unwrap();
    let target = directory.path().join("custom.png");

    Command::cargo_bin("imatree")
        .unwrap()
        .args(["--name", "Hello", "--output", target.to_str().unwrap()])
        .assert()
        .success();

    assert!(target.is_file());
    assert!(!directory.path().join("hello.png").exists());
}

#[test]
fn creates_missing_directories_on_request() {
    let directory = TempDir::new().unwrap();
    let nested = directory.path().join("deep/nested");

    Command::cargo_bin("imatree")
        .unwrap()
        .args([
            "--name",
            "Hello",
            "--path",
            nested.to_str().unwrap(),
            "--create-dirs",
        ])
        .assert()
        .success();

    assert!(nested.join("hello.png").is_file());
}

#[test]
fn a_missing_directory_is_reported_before_rendering() {
    let directory = TempDir::new().unwrap();
    let missing = directory.path().join("nowhere");

    Command::cargo_bin("imatree")
        .unwrap()
        .args(["--name", "Hello", "--path", missing.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("nowhere"))
        .stderr(predicate::str::contains("--create-dirs"));
}

#[test]
fn an_unknown_color_is_reported_without_redundant_context() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello", "--color", "purpleish"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "'purpleish' is not a valid color name or RGB hex value",
        ))
        .stderr(predicate::str::contains("invalid text style").not());
}

#[test]
fn a_blank_caption_is_rejected() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "   "])
        .assert()
        .failure()
        .stderr(predicate::str::contains("caption must contain text"));
}

#[test]
fn a_zero_font_size_is_rejected() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello", "--font-size", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "font size must be greater than zero",
        ));
}

#[test]
fn transparent_backgrounds_reach_the_saved_file() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello", "--background", "transparent"])
        .assert()
        .success();

    let image = image::open(directory.path().join("hello.png")).unwrap();

    assert_eq!(image.color(), image::ColorType::Rgba8);
}

#[test]
fn the_short_height_flag_is_available() {
    let directory = TempDir::new().unwrap();

    imatree(&directory)
        .args(["--name", "Hello", "-w", "300", "-H", "200"])
        .assert()
        .success();

    let image = image::open(directory.path().join("hello.png")).unwrap();

    assert_eq!((image.width(), image.height()), (300, 200));
}
