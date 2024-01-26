fn main() -> shadow_rs::SdResult<()> {
	let profile = std::env::var("PROFILE")?;
	if profile == "release" {
		println!("cargo:rerun-if-changed=.");
	}

	shadow_rs::new()
}
