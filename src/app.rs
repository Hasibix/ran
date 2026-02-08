#![allow(dead_code)]

// Imports

use std::fmt::{Display, Formatter, Result};
use serde::{ Serialize, Deserialize };
use walkdir::WalkDir;
use crate::{error::{Err, LE}, helpers::sandwich_args, launcher::{Launcher, ResolvedParts}};
use std::{collections::HashMap, path::PathBuf};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::*;

// Definitions

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

// Implementations

impl Display for App {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let mut table = Table::new();

		// 1. Setup Table Styling
		table
		.load_preset(UTF8_FULL_CONDENSED)
		.apply_modifier(UTF8_ROUND_CORNERS)
		.set_content_arrangement(ContentArrangement::Dynamic); // THIS IS THE MAGIC
		// It auto-detects terminal width and wraps text to fit!

		// 2. Add Meta Information
		if let Some(meta) = &self.meta {
			table.add_row(vec![
				Cell::new("Application").add_attribute(Attribute::Bold),
						  Cell::new(meta.name.as_deref().unwrap_or("Unknown")),
			]);
			if let Some(desc) = &meta.description {
				table.add_row(vec!["Description", desc]);
			}
		}

		// 3. Execution Section
		table.add_row(vec![
			Cell::new("Executable").fg(Color::Green),
					  Cell::new(&self.exec.bin),
		]);

		let args = if self.exec.args.is_empty() {
			"(none)".to_string()
		} else {
			self.exec.args.join(" ")
		};
		table.add_row(vec!["Arguments", &args]);

		// 4. Local Environment
		if let Some(env) = &self.env {
			for (name, value) in env {
				table.add_row(vec![
					Cell::new(format!("${}", name)).fg(Color::Yellow),
							  Cell::new(value)
				]);
			}
		}

		write!(f, "{}", table)
	}
}

impl App {
	pub fn find_all(root_path: &PathBuf) -> HashMap<String, PathBuf> {
		let mut apps = HashMap::new();
		let apps_dir = root_path.join("apps");

		if !apps_dir.exists() {
			return apps;
		}

		// Walk through the apps directory
		for entry in WalkDir::new(apps_dir)
			.min_depth(1) // Don't include the apps folder itself
			.into_iter()
			.filter_map(|e| e.ok())
			{
				let path = entry.path();

				// 1. Only care about .toml files
				if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {

					// 2. Sanitize the Name/Key
					// We want the path relative to the "apps" folder, without the .toml
					// e.g., "apps/games/doom.toml" -> "games/doom"
					if let Ok(relative_path) = path.strip_prefix(&root_path.join("apps")) {
						let mut name = relative_path.to_string_lossy().to_string();

						// Remove .toml extension
						if name.ends_with(".toml") {
							name.truncate(name.len() - 5);
						}

						// 3. Normalize slashes and trim
						let sanitized_name = name
						.replace('\\', "/") // Ensure cross-platform consistency
						.trim_matches('/')
						.to_string();

						apps.insert(sanitized_name, path.to_path_buf());
					}
				}
			}
			apps
	}

	pub fn resolve_recursive(&self, launcher: &Launcher) -> Err<ResolvedParts> {
		let mut parts = if self.exec.bin.starts_with('@') {
			if self.exec.bin.len() < 2 {
				return Err(LE::InvalidApp("".into(), Some("App name cannot be empty!".into())));
			}
			let runner_name = &self.exec.bin[1..];
			let runner_app = launcher.load_app(runner_name)?;
			runner_app.resolve_recursive(launcher)
		} else {
			// Base case: No more runners
			Ok(ResolvedParts {
				bin: self.exec.bin.clone(),
			   args: Vec::new(),
			   env: HashMap::new(),
			})
		}?;

		// 2. Handle args
		if !parts.args.is_empty() {
			parts.args = sandwich_args(parts.args, self.exec.args.clone());
		} else {
			parts.args = self.exec.args.clone();
		}

		// 3. Merge env
		if let Some(e) = &self.env {
			parts.env.extend(e.clone());
		}

		Ok(parts)
	}
}
