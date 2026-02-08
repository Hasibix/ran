#![allow(dead_code)]

// Imports

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::app::App;
use crate::config::Config;
use crate::error::{Err, LE};
use crate::helpers::{app_resolver, sandwich_args, expand_vars};

// Definitions

pub struct ResolvedParts {
	pub bin: String,
	pub args: Vec<String>,
	pub env: HashMap<String, String>,
}

pub struct Launcher {
	pub apps: HashMap<String, PathBuf>,
	pub config: Config,
}

// Implementations

impl Launcher {
	pub fn resolve_alias_chain(&self, start_key: &str) -> Err<Vec<String>> {
		let mut chain = vec![start_key.to_string()];
		let mut current = start_key;

		// Keep looking up while the value exists in the alias map
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

	pub fn find_app(&self, query: &str) -> Err<&PathBuf> {
		self.find_app_inner(query, vec![])
	}

	pub fn load_app(&self, query: &str) -> Err<App> {
		let path = self.find_app(query)?;
		let content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&content)?)
	}

	pub fn load_app_from(&self, path: &PathBuf) -> Err<App> {
		let content = std::fs::read_to_string(path)?;
		Ok(toml::from_str(&content)?)
	}

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

	pub fn launch_app(
		&self,
		name: &str,
		cli_args: Vec<String>,
		cli_env: HashMap<String, String>,
		background: bool
	) -> Err<()> {
		// 1. Resolve @chain
		let target_app = self.load_app(name)?;
		let parts = target_app.resolve_recursive(self)?;

		// 2. Sandwich args (%! replacement)
		let intermediate_args = sandwich_args(parts.args, cli_args);

		// 3. Layer Envs
		let mut final_env = cli_env;
		final_env.extend(self.config.env.clone());
		final_env.extend(parts.env);

		// 4. Resolve %vars% (Only on what we are about to use)
		let final_bin = expand_vars(&parts.bin, self);

		let final_args: Vec<String> = intermediate_args.into_iter()
		.map(|arg| expand_vars(&arg, self))
		.collect();

		let final_env: HashMap<String, String> = final_env.into_iter()
		.map(|(k, v)| (k, expand_vars(&v, self)))
		.collect();

		// 5. Build and Launch

		if background {
			let (term, args) = if let Some((term, args)) = self.config.terminal_runner.split_once(' ') {
				(term.to_string(), args.to_string())
			} else {
				(self.config.terminal_runner.clone(), String::new())
			};
			if args.matches("%!").count() != 1 {
				return Err(LE::InvalidConfig(Some("Expected one \"%!\" in config.terminal_runner".into())))
			}
			let t_args_vec: Vec<String> = args
			.split(' ')
			.filter(|s| !s.is_empty()) // Clean up extra spaces
			.map(|s| s.to_string())
			.collect();

			let mut command_to_run = vec![final_bin]; // Start with the binary
			command_to_run.extend(final_args);

			let mut cmd = Command::new(term);
			cmd.args(sandwich_args(t_args_vec, command_to_run)).envs(final_env);
			// "Fire and Forget"
			cmd.spawn()?;
			println!("🚀 Launched {} in background in default terminal.", name);
		} else {
			let mut cmd = Command::new(final_bin);
			cmd.args(final_args).envs(final_env);
			// Wait for exit
			println!("🚀 Launching app {}...", name);
			let status = cmd.status()?;
			if !status.success() {
				eprintln!("⚠️ Process exited with: {}", status);
			}
		}
		Ok(())
	}
}
