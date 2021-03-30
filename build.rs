use std::process::Command;
fn main() {
    let git_rev_no = Command::new("git")
        .args(&["rev-list", "--count", "HEAD"])
        .output()
        .unwrap();
    let git_rev_no = String::from_utf8(git_rev_no.stdout).unwrap();
    let git_commit_hash = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_commit_hash = String::from_utf8(git_commit_hash.stdout).unwrap();
    println!("cargo:rustc-env=GIT_REV={}", git_rev_no);
    println!("cargo:rustc-env=GIT_HASH={}", git_commit_hash);
}
