#![allow(dead_code)]

// --- imports ---
use std::fmt::{Display, Formatter, Result};
use indexmap::IndexMap;
use serde::{ Serialize, Deserialize };
use walkdir::WalkDir;
use crate::{error::{Err, LE}, utils::{generate_rows, make_box, sandwich_args}, launcher::{Launcher, ResolvedParts}};
use std::{collections::HashMap, path::PathBuf};
use colored::*;

// --- definitions ---
#[derive(Serialize, Deserialize)]
pub struct App {
	pub meta: Option<Meta>,
	pub exec: Exec,
	pub env: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Meta {
	pub name: Option<String>,
	pub description: Option<String>,
	pub version: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Exec {
	pub bin: String,
	pub args: Vec<String>,
}

// --- implementations ---
impl Display for App {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let mut sections: IndexMap<String, IndexMap<String, String>> = IndexMap::new();

		// --- metadata ---
		if let Some(meta) = &self.meta {
			let mut meta_map = IndexMap::new();

			let name = meta.name.as_deref().unwrap_or("Unspecified");
			let version = meta.version.as_deref().unwrap_or("Unspecified");

			meta_map.insert("Name".bright_yellow().to_string(), name.into());
			meta_map.insert("Version".bright_yellow().to_string(), version.into());
			if let Some(desc) = &meta.description {
				meta_map.insert("Description".bright_yellow().to_string(), desc.into());
			}

			sections.insert(format!("{}", "Metadata".bright_yellow().bold()), meta_map);
		}

		// --- execution ---
		let mut exec_map = IndexMap::new();
		exec_map.insert("Exectuable".bright_green().to_string(), self.exec.bin.clone());

		if !self.exec.args.is_empty() {
			exec_map.insert(
				"Arguments".bright_green().to_string(),
				self.exec.args
				.iter()
				.map(|a| {
					if a.contains(' ') || a.contains('"') {
						format!("\"{}\"", a.replace('"', "\\\""))
					} else {
						a.clone()
					}
				})
				.collect::<Vec<_>>()
				.join(" ")
			);
		}

		sections.insert(format!("{}", "Execution".bright_green().bold()), exec_map);

		// --- local environment ---
		let mut env_map = IndexMap::new();
		if let Some(env) = &self.env {
			if env.is_empty() {
				env_map.insert("(no local environment overrides provided)".bright_blue().to_string(), "".into());
			}
			for (name, value) in env {
				env_map.insert(format!("${}", name).bright_blue().to_string(), value.clone());
			}
		}
		if !env_map.is_empty() {
			sections.insert(format!("{}", "Local Environment".bright_blue().bold()), env_map);
		}

		// generate rows and make box
		let rows = generate_rows(sections);
		make_box(f, "App Info", rows)?;

		Ok(())
	}
}

impl App {
	/// finds all app definitions in {root_path}/apps and returns a map of app name -> path to definition
	pub fn find_all(root_path: &PathBuf) -> HashMap<String, PathBuf> {
		let mut apps = HashMap::new();
		let apps_dir = root_path.join("apps");

		if !apps_dir.exists() {
			return apps;
		}

		// walk through the apps directory
		for entry in WalkDir::new(apps_dir)
			.min_depth(1) // don't include the apps folder itself
			.into_iter()
			.filter_map(|e| e.ok())
			{
				let path = entry.path();

				// only care about .toml files
				if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {

					// sanitize the name/key
					// we want the path relative to the "apps" folder, without the .toml
					// e.g., "apps/games/doom.toml" -> "games/doom"
					if let Ok(relative_path) = path.strip_prefix(&root_path.join("apps")) {
						let mut name = relative_path.to_string_lossy().to_string();

						// remove .toml extension
						if name.ends_with(".toml") {
							name.truncate(name.len() - 5);
						}

						// normalize slashes and trim
						let sanitized_name = name
						.replace('\\', "/") // ensure cross-platform consistency
						.trim_matches('/')
						.to_string();

						apps.insert(sanitized_name, path.to_path_buf());
					}
				}
			}
			apps
	}

	/// resolves the app's executable, arguments, and environment variables, including any nested runners (if the executable starts with '@')
	pub fn resolve_recursive(&self, launcher: &Launcher) -> Err<ResolvedParts> {
		let mut parts = if self.exec.bin.starts_with('@') {
			if self.exec.bin.len() < 2 {
				return Err(LE::InvalidApp("".into(), Some("app name cannot be empty!".into())));
			}
			let runner_name = &self.exec.bin[1..];
			let runner_app = launcher.load_app(runner_name)?;
			runner_app.resolve_recursive(launcher)
		} else {
			// base case: no more runners
			Ok(ResolvedParts {
				bin: self.exec.bin.clone(),
				args: Vec::new(),
				env: HashMap::new(),
			})
		}?;

		// handle args
		if !parts.args.is_empty() {
			parts.args = sandwich_args(parts.args, self.exec.args.clone());
		} else {
			parts.args = self.exec.args.clone();
		}

		// merge env overrides
		if let Some(e) = &self.env {
			parts.env.extend(e.clone());
		}

		Ok(parts)
	}
}
