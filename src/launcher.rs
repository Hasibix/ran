#![allow(dead_code)]

// --- imports ---
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::app::App;
use crate::config::Config;
use crate::error::{Err, LE};
use crate::utils::{app_resolver, sandwich_args, expand_vars};

// --- definitions ---
pub struct ResolvedParts {
	pub bin: String,
	pub args: Vec<String>,
	pub env: HashMap<String, String>,
}

pub struct Launcher {
	pub apps: HashMap<String, PathBuf>,
	pub config: Config,
}

// --- implementations ---
impl Launcher {
	/// resolves alias chain, and errors on circular references, returning the full chain for better error messages
	pub fn resolve_alias_chain(&self, start_key: &str) -> Err<Vec<String>> {
		let mut chain = vec![start_key.to_string()];
		let mut current = start_key;

		// keep looking up while the value exists in the alias map
		// and avoid infinite loops
		while let Some(next) = &self.config.alias.get(current) {
			if chain.contains(next) {
				chain.push(next.to_string());
				return Err(LE::CircularAlias(chain))
			}
			chain.push(next.to_string());
			current = next;
		}
		Ok(chain)
	}

	/// (private) finds app from query with stack tracking
	fn find_app_inner(&self, query: &str, stack: Vec<String>) -> Err<&PathBuf> {
		if stack.contains(&query.into()) {
			let mut stack = stack;
			stack.push(query.into());
			return Err(LE::CircularAlias(stack));
		}
		let query = query.trim().trim_matches('/');
		if query.is_empty() { return Err(LE::AppNotFound(query.into())); }

		if let Some(app) = self.config.alias.get(query) {
			let mut stack = stack;
			stack.push(query.to_string());
			return self.find_app_inner(app, stack);
		}

		let matches: Vec<&PathBuf> = self.apps.iter()
		.filter(|(full_name, _)| {
			let leaf_name = full_name.split('/').last().unwrap_or(full_name);
			full_name == &query || leaf_name == query
		})
		.map(|(_, path)| path)
		.collect();

		if matches.len() > 0 {
			match matches.len() {
				1 => Ok(matches.get(0).ok_or(LE::AppNotFound(query.into()))?),
				_ => Ok(app_resolver(self, query, matches)?)
			}
		} else {
			Err(LE::AppNotFound(query.into()))
		}
	}

	/// finds app from query, resolving aliases, and errors on circular references
	pub fn find_app(&self, query: &str) -> Err<&PathBuf> {
		self.find_app_inner(query, vec![])
	}

	/// loads app from query, resolving aliases, and errors on circular references
	pub fn load_app(&self, query: &str) -> Err<App> {
		let path = self.find_app(query)?;
		let content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&content)?)
	}

	/// loads app from path, without resolving aliases
	pub fn load_app_from(&self, path: &PathBuf) -> Err<App> {
		let content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&content)?)
	}

	/// initializes launcher by scanning for apps and loading config
	pub fn init(path: &PathBuf) -> Err<Launcher> {
		let apps = App::find_all(path);

		let config = std::fs::read_to_string(path.join("config.toml"))
		.map_err(LE::from)
		.and_then(|c| toml::from_str(&c).map_err(LE::from))?;

		Ok(Launcher {
			apps,
			config,
		})
	}

	/// launches an app by query with cli args and env, resolving aliases, and errors on circular references
	pub fn launch_app(
		&self,
		query: &str,
		cli_args: Vec<String>,
		cli_env: HashMap<String, String>,
		background: bool
	) -> Err<()> {
		// 1. resolve @chain
		let path = self.find_app(query)?;
		let name = self.apps.iter().find(|(_, p)| *p == path).map(|(n, _)| n).ok_or(LE::AppNotFound(query.into()))?;
		let target_app = self.load_app_from(path)?;
		let parts = target_app.resolve_recursive(self)?;

		// 2. sandwich args (%! replacement)
		let intermediate_args = sandwich_args(parts.args, cli_args);

		// 3. layer envs
		let mut final_env = cli_env;
		final_env.extend(self.config.env.clone());
		final_env.extend(parts.env);

		// 4. resolve %vars% (only on what we are about to use)
		let final_bin = expand_vars(&parts.bin, self);

		let final_args: Vec<String> = intermediate_args.into_iter()
		.map(|arg| expand_vars(&arg, self))
		.collect();

		let final_env: HashMap<String, String> = final_env.into_iter()
		.map(|(k, v)| (k, expand_vars(&v, self)))
		.collect();

		// 5. build and launch
		if background {
			let mut cmd = Command::new(final_bin);
			cmd.args(final_args)
				.stdin(Stdio::null())
				.stdout(Stdio::null())
				.stderr(Stdio::null());
			// spawn and immediately forget
			let _ = cmd.spawn();
			println!("launched app {} in background!", name);
		} else {
			let mut cmd = Command::new(final_bin);
			cmd.args(final_args).envs(final_env);
			// wait for exit
			println!("launching app {}...", name);
			let status = cmd.status()?;
			if !status.success() {
				eprintln!("process exited with: {}", status);
			}
		}
		Ok(())
	}
}
