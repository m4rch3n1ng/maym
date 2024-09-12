use clap::{Parser, Subcommand};
use shadow_rs::{concatcp, shadow};

const fn version() -> &'static str {
	shadow!(build);
	concatcp!("v", build::PKG_VERSION, ", commit ", build::SHORT_COMMIT)
}

#[derive(Debug, Parser)]
#[command(author, version = version(), about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
	#[clap(subcommand)]
	pub cmd: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
	#[command(about = "edit config")]
	Config,
}

#[cfg(test)]
#[test]
fn verify() {
	use clap::CommandFactory;

	let cmd = Cli::command();
	cmd.debug_assert();
}
