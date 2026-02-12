// --- imports ---
use indexmap::IndexMap;
use serde::{Serialize, Deserialize};
use std::default::Default;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use colored::Colorize;

use crate::utils::{generate_rows, make_box};

// --- definitions ---
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
	#[serde(default = "default_interactive")]
	pub interactive: bool,
	pub editor: Option<String>,

	#[serde(default)]
	pub alias: HashMap<String, String>,

	#[serde(default)]
	pub vars: HashMap<String, String>,

	#[serde(default)]
	pub env: HashMap<String, String>,
}

// --- implementations ---
impl Display for Config {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let mut sections: IndexMap<String, IndexMap<String, String>> = IndexMap::new();

		// --- general settings ---
		let mut general = IndexMap::new();
		general.insert("Interactive".bright_cyan().to_string(), self.interactive.to_string());
		if let Some(editor) = &self.editor {
			general.insert("Editor".bright_cyan().to_string(), editor.clone());
		} else {
			general.insert("Editor".bright_cyan().to_string(), "(not specified)".bright_black().to_string());
		}
		sections.insert(format!("{}", "General Settings".bright_cyan().bold()), general);

		// --- app aliases ---
		let mut aliases = IndexMap::new();
		if self.alias.is_empty() {
			aliases.insert("(no app aliases provided)".bright_magenta().to_string(), "".into());
		} else {
			for (name, target) in &self.alias {
				aliases.insert(name.bright_magenta().to_string(), target.clone());
			}
		}
		sections.insert(format!("{}", "App Aliases".bright_magenta().bold()), aliases);

		// --- custom variables ---
		let mut vars = IndexMap::new();
		if self.vars.is_empty() {
			vars.insert("(no custom variables provided)".bright_red().to_string(), "".into());
		} else {
			for (name, value) in &self.vars {
				vars.insert(format!("%{}%", name).bright_red().to_string(), value.clone());
			}
		}
		sections.insert(format!("{}", "Custom Variables".bright_red().bold()), vars);

		// --- global environment ---
		let mut env = IndexMap::new();
		if self.env.is_empty() {
			env.insert("(no environment overrides provided)".bright_blue().to_string(), "".into());
		} else {
			for (name, value) in &self.env {
				env.insert(format!("${}", name).bright_blue().to_string(), value.clone());
			}
		}
		sections.insert(format!("{}", "Global Environment".bright_blue().bold()), env);

		// generate rows and make box
		let rows = generate_rows(sections);
		make_box(f, "Config Info", rows)?;

		Ok(())
	}
}

// --- functions ---
pub fn default_interactive() -> bool { true }
