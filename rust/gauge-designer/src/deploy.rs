//! Deploy to the device over ssh/scp, with the same settings (and
//! environment variables) as `deploy-arm64.sh`.

use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub struct DeployTarget {
	pub host: String,
	pub user: String,
	/// Used through `sshpass` when it's installed; otherwise ssh uses your key.
	pub password: String,
	/// The Rust app's folder (`BIN_DIR`); `gauges.json` lives here.
	pub bin_dir: String,
}

impl DeployTarget {
	/// Defaults from `REMOTE_HOST`, `REMOTE_USER`, `REMOTE_PASS`, `BIN_ROOT` and `BIN_DIR`.
	/// Unlike `deploy-arm64.sh` there's no default password: set `REMOTE_PASS`
	/// or type it in the Deploy window, or leave it empty to use your ssh key.
	pub fn from_env() -> Self {
		let env = |k: &str, d: &str| std::env::var(k).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| d.to_string());
		let bin_root = env("BIN_ROOT", "/mnt/mmc/ports");
		Self {
			host: env("REMOTE_HOST", "192.168.0.234"),
			user: env("REMOTE_USER", "root"),
			password: std::env::var("REMOTE_PASS").unwrap_or_default(),
			bin_dir: env("BIN_DIR", &format!("{bin_root}/GaugeRust")),
		}
	}

	fn login(&self) -> String {
		format!("{}@{}", self.user, self.host)
	}

	fn command(&self, program: &str) -> Command {
		let use_sshpass = !self.password.is_empty() && which("sshpass");
		let mut cmd = if use_sshpass {
			// `-e` reads it from SSHPASS, so it's never on the command line
			// (visible in `ps`) or in the log.
			let mut c = Command::new("sshpass");
			c.arg("-e").arg(program).env("SSHPASS", &self.password);
			c
		} else {
			Command::new(program)
		};
		cmd.args(["-o", "StrictHostKeyChecking=no", "-o", "ConnectTimeout=10"]);
		cmd
	}

	fn run(&self, mut cmd: Command, log: &mut String) -> Result<(), String> {
		log.push_str(&format!("$ {}\n", describe(&cmd)));
		let out = cmd.output().map_err(|e| format!("{}: {e}", cmd.get_program().to_string_lossy()))?;
		log.push_str(&String::from_utf8_lossy(&out.stdout));
		log.push_str(&String::from_utf8_lossy(&out.stderr));
		if out.status.success() {
			Ok(())
		} else {
			Err(format!("{} failed ({})", cmd.get_program().to_string_lossy(), out.status))
		}
	}

	fn mkdir(&self, dirs: &[String], log: &mut String) -> Result<(), String> {
		let mut cmd = self.command("ssh");
		let quoted: Vec<String> = dirs.iter().map(|d| shell_quote(d)).collect();
		cmd.arg(self.login()).arg(format!("mkdir -p {}", quoted.join(" ")));
		self.run(cmd, log)
	}

	fn copy(&self, local: &Path, remote_dir: &str, log: &mut String) -> Result<(), String> {
		let mut cmd = self.command("scp");
		cmd.arg(local).arg(format!("{}:{}/", self.login(), remote_dir));
		self.run(cmd, log)
	}

	/// Uploads a saved gauge and the images it uses to
	/// `<bin_dir>/<subdir>/`. Returns the log and the `CustomDefinitionFile`
	/// value that points `gauges.json` at it.
	pub fn upload_gauge(
		&self,
		json: &Path,
		images: &[PathBuf],
		subdir: &str,
	) -> Result<(String, String), (String, String)> {
		let mut log = String::new();
		let file_name = json.file_name().and_then(|n| n.to_str()).unwrap_or("gauge.json").to_string();
		let subdir = subdir.trim_matches('/');
		let dir = if subdir.is_empty() { self.bin_dir.clone() } else { format!("{}/{subdir}", self.bin_dir) };
		let result = (|| {
			let mut dirs = vec![dir.clone()];
			if !images.is_empty() {
				dirs.push(format!("{dir}/images"));
			}
			self.mkdir(&dirs, &mut log)?;
			self.copy(json, &dir, &mut log)?;
			for image in images {
				self.copy(image, &format!("{dir}/images"), &mut log)?;
			}
			Ok(())
		})();
		let config_path = if subdir.is_empty() { file_name } else { format!("{subdir}/{file_name}") };
		match result {
			Ok(()) => Ok((log, config_path)),
			Err(e) => Err((log, e)),
		}
	}

	/// Uploads `gauges.json` into the app folder.
	pub fn upload_config(&self, config: &Path) -> Result<String, (String, String)> {
		let mut log = String::new();
		let result = self.mkdir(std::slice::from_ref(&self.bin_dir), &mut log).and_then(|_| {
			let mut cmd = self.command("scp");
			cmd.arg(config).arg(format!("{}:{}/gauges.json", self.login(), self.bin_dir));
			self.run(cmd, &mut log)
		});
		match result {
			Ok(()) => Ok(log),
			Err(e) => Err((log, e)),
		}
	}
}

/// Images a saved definition references inside its `images/` folder.
pub fn packaged_images(def: &gauge_render::CustomGaugeDefinition, json: &Path) -> Vec<PathBuf> {
	let dir = json.parent().unwrap_or(Path::new("."));
	let mut paths: Vec<&str> = def.background_image.as_deref().into_iter().collect();
	for e in &def.elements {
		match e {
			gauge_render::Element::Needle(n) => paths.extend(n.image_path.as_deref()),
			gauge_render::Element::Image(i) => paths.push(&i.image_path),
			gauge_render::Element::Gif(g) => paths.push(&g.image_path),
			_ => {}
		}
	}
	paths.into_iter().filter(|p| p.starts_with("images/")).map(|p| dir.join(p)).filter(|p| p.is_file()).collect()
}

fn which(program: &str) -> bool {
	std::env::var_os("PATH").is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join(program).is_file()))
}

fn shell_quote(s: &str) -> String {
	format!("'{}'", s.replace('\'', r"'\''"))
}

/// The command line for the log.
fn describe(cmd: &Command) -> String {
	let mut parts = vec![cmd.get_program().to_string_lossy().to_string()];
	parts.extend(cmd.get_args().map(|a| a.to_string_lossy().to_string()));
	parts.join(" ")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn quotes_for_the_remote_shell() {
		assert_eq!(shell_quote("/mnt/mmc/ports/Gauge Rust"), "'/mnt/mmc/ports/Gauge Rust'");
		assert_eq!(shell_quote("it's"), r"'it'\''s'");
	}

	#[test]
	fn keeps_the_password_off_the_command_line() {
		// Not a literal, so it isn't mistaken for a real credential.
		let password = format!("pw{}", std::process::id());
		let target = DeployTarget { host: "h".into(), user: "u".into(), password, bin_dir: "/d".into() };
		let mut cmd = Command::new("sshpass");
		cmd.arg("-e").arg("ssh").env("SSHPASS", &target.password);
		assert!(!describe(&cmd).contains(&target.password));
	}
}
