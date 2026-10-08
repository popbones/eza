// SPDX-FileCopyrightText: 2026 Popbones
// SPDX-License-Identifier: EUPL-1.2
#![cfg(target_os = "macos")]

use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "eza-macos-hidden-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, "").unwrap();
        path
    }

    fn dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        path
    }

    fn list(&self, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_eza"))
            .current_dir(&self.0)
            .env_clear()
            .args(["--color=never", "--icons=never", "--oneline"])
            .args(args)
            .arg(".")
            .output()
            .unwrap();
        assert!(output.status.success(), "{:?}", output);
        String::from_utf8(output.stdout).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn hidden_flag(path: &Path) {
    // -h changes a symlink's own flags, matching Finder's entry visibility.
    assert!(
        Command::new("chflags")
            .args(["-h", "hidden"])
            .arg(path)
            .status()
            .unwrap()
            .success()
    );
}

fn finder_flags(path: &Path, flags: u16) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let mut info = [0_u8; 32];
    info[8..10].copy_from_slice(&flags.to_be_bytes());
    // SAFETY: the strings and the 32-byte readable buffer have the required lifetimes.
    assert_eq!(
        unsafe {
            libc::setxattr(
                path.as_ptr(),
                c"com.apple.FinderInfo".as_ptr(),
                info.as_ptr().cast(),
                info.len(),
                0,
                libc::XATTR_NOFOLLOW,
            )
        },
        0
    );
}

#[test]
fn hides_both_macos_flags_and_dotfiles_but_keeps_other_finder_flags() {
    let fixture = Fixture::new();
    fixture.file("visible");
    fixture.file(".dotfile");
    hidden_flag(&fixture.file("bsd-hidden"));
    finder_flags(&fixture.file("finder-hidden"), 0x4000);
    let icon = fixture.file("Icon\r");
    hidden_flag(&icon);
    finder_flags(&icon, 0x4000);
    finder_flags(&fixture.file("custom-icon-visible"), 0x0400);
    hidden_flag(&fixture.dir("bsd-hidden-dir"));
    finder_flags(&fixture.dir("finder-hidden-dir"), 0x4000);

    assert_eq!(fixture.list(&[]), "custom-icon-visible\nvisible\n");
    for args in [
        vec!["-a"],
        vec!["-A"],
        vec!["--all"],
        vec!["--almost-all"],
        vec!["-aa"],
        vec!["-la"],
    ] {
        let output = fixture.list(&args);
        for name in [".dotfile", "bsd-hidden", "finder-hidden", "Icon"] {
            assert!(output.contains(name), "{args:?}: {output}");
        }
    }
}

#[test]
fn filters_hidden_subtrees_and_nested_entries_in_recursive_and_tree_views() {
    let fixture = Fixture::new();
    let hidden_dir = fixture.dir("hidden-dir");
    hidden_flag(&hidden_dir);
    fs::write(hidden_dir.join("inside-hidden-dir"), "").unwrap();
    let visible_dir = fixture.dir("visible-dir");
    let hidden_child = visible_dir.join("nested-hidden");
    fs::write(&hidden_child, "").unwrap();
    finder_flags(&hidden_child, 0x4000);
    fs::write(visible_dir.join("nested-visible"), "").unwrap();

    for mode in ["-R", "-T"] {
        let output = fixture.list(&[mode]);
        assert!(output.contains("nested-visible"), "{output}");
        assert!(!output.contains("hidden"), "{output}");
        let output = fixture.list(&[mode, "-a"]);
        assert!(output.contains("inside-hidden-dir"), "{output}");
        assert!(output.contains("nested-hidden"), "{output}");
    }
}

#[test]
fn explicitly_named_hidden_files_and_directories_remain_accessible() {
    let fixture = Fixture::new();
    hidden_flag(&fixture.file("hidden-file"));
    let dir = fixture.dir("hidden-dir");
    finder_flags(&dir, 0x4000);
    fs::write(dir.join("inside"), "").unwrap();

    for (args, expected) in [
        (vec!["hidden-file"], "hidden-file"),
        (vec!["hidden-dir"], "inside"),
        (vec!["-d", "hidden-dir"], "hidden-dir"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_eza"))
            .current_dir(&fixture.0)
            .env_clear()
            .arg("--color=never")
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8(output.stdout).unwrap().contains(expected));
    }
}

#[test]
fn symlink_visibility_uses_the_entry_even_when_dereferencing() {
    let fixture = Fixture::new();
    hidden_flag(&fixture.file("hidden-target"));
    finder_flags(&fixture.file("finder-target"), 0x4000);
    symlink("hidden-target", fixture.0.join("visible-link")).unwrap();
    symlink("finder-target", fixture.0.join("visible-finder-link")).unwrap();
    fixture.file("visible-target");
    let hidden_link = fixture.0.join("hidden-link");
    symlink("visible-target", &hidden_link).unwrap();
    hidden_flag(&hidden_link);
    symlink("missing", fixture.0.join("broken-link")).unwrap();

    for args in [vec![], vec!["-X"]] {
        let output = fixture.list(&args);
        assert!(output.contains("visible-link"), "{output}");
        assert!(output.contains("visible-finder-link"), "{output}");
        assert!(output.contains("broken-link"), "{output}");
        assert!(
            !output.lines().any(|line| line == "hidden-link"),
            "{output}"
        );
    }
    assert!(fixture.list(&["-a"]).contains("hidden-link"));
}
