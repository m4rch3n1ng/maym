use std::{path::Path, process::Command};

fn git(args: &[&str]) -> Option<String> {
	(Command::new("git").args(args).output().ok())
		.and_then(|output| String::from_utf8(output.stdout).ok())
		.map(|output| output.trim().to_owned())
}

fn main() {
	let version = env!("CARGO_PKG_VERSION");
	let hash = git(&["rev-parse", "--short", "HEAD"]);
	let hash = hash.as_deref().unwrap_or("unknown commit");

	println!("cargo::rustc-env=MAYM_VERSION={version} ({hash})");

	// get the path to the .git directory
	let Some(dir) = git(&["rev-parse", "--git-dir"]) else {
		return;
	};

	// rerun if .git/HEAD changed, if head starts pointing at something else
	// (e.g. when switching branches)
	let head = Path::new(&dir).join("HEAD");
	if head.exists() {
		println!("cargo::rerun-if-changed={}", head.display());
	}

	// rerun if the current symbolic ref / current branch changes
	// (e.g. when adding / amending a commit to the current branch)
	let Some(head_ref) = git(&["symbolic-ref", "HEAD"]) else {
		return;
	};
	let head_ref = Path::new(&dir).join(head_ref);
	if head_ref.exists() {
		println!("cargo::rerun-if-changed={}", head_ref.display());
	}
}
