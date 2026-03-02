// --- constants ---
pub const DEFAULT_CONFIG: &str = include_str!("../res/config.toml");
pub const DEFAULT_CONFIG_CLEAN: &str = include_str!("../res/config_clean.toml");

// --- imports ---
use anyhow::{anyhow, bail, Context, Result};
use colored::Colorize;
use indexmap::IndexMap as Map;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::Path;
use toml_edit::{table, value, DocumentMut};
use crate::util::args::parse_bool;
use crate::util::table::*;

// --- functions ---
pub fn new_config_file(config_file: &Path, clean: bool) -> Result<()> {
	if let Some(parent) = config_file.parent() {
		fs::create_dir_all(parent)?;
	}
	if config_file.exists() {
		return Ok(());
	}
	fs::write(config_file, match clean {
		true => DEFAULT_CONFIG_CLEAN,
		false => DEFAULT_CONFIG,
	})?;
	Ok(())
}

// --- definitions ---
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
	#[serde(default)]
	pub noninteractive: bool,
	pub alias: Option<Map<String, String>>,
	pub vars: Option<Map<String, String>>,
	pub env: Option<Map<String, String>>,
}

// --- implementations ---
impl Config {
	/// loads config from toml (fails if config file doesn't exist)
	pub fn load(config_file: &Path) -> Result<Self> {
		if !config_file.exists() {
			bail!("config file does not exist in '{}'", config_file.display());
		}

		let config_str = std::fs::read_to_string(config_file)?;
		let config: Config = toml::from_str(&config_str)?;

		Ok(config)
	}

	/// saves config file with proper formatting
	pub fn save(&self, config_file: &Path) -> Result<()> {
		// ensure the directory exists
		if let Some(parent) = config_file.parent() {
			fs::create_dir_all(parent)?;
		}

		// read the existing file or start with empty doc
		let mut doc = if config_file.exists() {
			let text = fs::read_to_string(config_file)
				.with_context(|| format!("failed to read config at '{}'", config_file.display()))?;
			text.parse::<DocumentMut>().context("failed to parse config toml")?
		} else {
			DocumentMut::new()
		};

		// 1. general config
		doc["noninteractive"] = value(self.noninteractive);

		// 2. alias
		if let Some(alias) = &self.alias {
			if !doc.as_table().contains_key("alias") {
				doc["alias"] = table();
			}
			let table = doc["alias"].as_table_mut().unwrap();
			table.clear();
			for (k, v) in alias { table[k] = value(v.clone()); }
		} else {
			doc.as_table_mut().remove("alias");
		}

		// 3. vars
		if let Some(vars) = &self.vars {
			if !doc.as_table().contains_key("vars") {
				doc["vars"] = table();
			}
			let table = doc["vars"].as_table_mut().unwrap();
			table.clear();
			for (k, v) in vars { table[k] = value(v.clone()); }
		} else {
			doc.as_table_mut().remove("vars");
		}

		// 4. env
		if let Some(env) = &self.env {
			if !doc.as_table().contains_key("env") {
				doc["env"] = table();
			}
			let table = doc["env"].as_table_mut().unwrap();
			table.clear();
			for (k, v) in env { table[k] = value(v.clone()); }
		} else {
			doc.as_table_mut().remove("env");
		}

		// 5. write back
		fs::write(config_file, doc.to_string())
			.with_context(|| format!("failed to write app to {:?}", config_file.display()))?;

		Ok(())
	}

	// getters and setters

	pub fn get(&self, query: &str) -> Option<String> {
		let parts: Vec<&str> = query.split('.').collect();
		self.get_slice(parts.as_slice())
	}

	pub fn set(&mut self, query: &str, value: String) -> Result<()> {
		let parts: Vec<&str> = query.split('.').collect();
		self.set_slice(parts.as_slice(), value)
	}

	pub fn unset(&mut self, query: &str) -> Result<()> {
		let parts: Vec<&str> = query.split('.').collect();
		self.unset_slice(parts.as_slice())
	}

	pub fn get_slice(&self, parts: &[&str]) -> Option<String> {
		match parts {
			["*"] => Some(format!("{}", self)),
			["noninteractive"] => Some(self.noninteractive.to_string()),
			["alias", k] => self.alias.as_ref()?.get(*k).cloned(),
			["vars", k] => self.vars.as_ref()?.get(*k).cloned(),
			["env", k] => self.env.as_ref()?.get(*k).cloned(),
			_ => None,
		}
	}

	pub fn set_slice(&mut self, parts: &[&str], value: String) -> Result<()> {
		match parts {
			["noninteractive"] => self.noninteractive = parse_bool(&value)
				.ok_or(anyhow!("parse error: '{value}' is not a boolean"))?,

			["alias", k] => {
				let alias = self.alias.get_or_insert_default();
				alias.insert((*k).to_string(), value);
			}

			["vars", k] => {
				let vars = self.vars.get_or_insert_default();
				vars.insert((*k).to_string(), value);
			}

			["env", k] => {
				let env = self.env.get_or_insert_default();
				env.insert((*k).to_string(), value);
			}

			_ => bail!("invalid key '{}'", parts.join(".")),
		};
		Ok(())
	}

	pub fn unset_slice(&mut self, parts: &[&str]) -> Result<()> {
		match parts {
			["*"] => {
				*self = Default::default();
			}
			["noninteractive"] => self.noninteractive = false,

			["alias", k] => match *k {
				"*" => {
					self.alias = None;
				}
				_ => {
					let alias = self.alias
						.as_mut()
						.ok_or_else(|| anyhow!("alias map not defined!"))?;
					alias.swap_remove(*k);
				}
			}

			["vars", k] => match *k {
				"*" => {
					self.vars = None;
				}
				_ => {
					let vars = self.vars
						.as_mut()
						.ok_or_else(|| anyhow!("vars map not defined!"))?;
					vars.swap_remove(*k);
				}
			}

			["env", k] => match *k {
				"*" => {
					self.env = None;
				}
				_ => {
					let env = self.env
						.as_mut()
						.ok_or_else(|| anyhow!("env map not defined!"))?;
					env.swap_remove(*k);
				}
			}

			_ => bail!("invalid key '{}'", parts.join(".")),
		};
		Ok(())
	}
}

impl Display for Config {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let mut sections: Map<String, Map<String, String>> = Map::new();

		// 1. general settings
		let mut general = Map::new();
		general.insert("Noninteractive".bright_cyan().to_string(), self.noninteractive.to_string());
		sections.insert(format!("{}", "General Settings".bright_cyan().bold()), general);

		// 2. app aliases
		let mut alias_map = Map::new();
		if let Some(alias) = &self.alias {
			if alias.is_empty() {
				alias_map.insert("(no app aliases provided)".bright_magenta().to_string(), "".into());
			} else {
				for (name, target) in alias {
					alias_map.insert(name.bright_magenta().to_string(), target.clone());
				}
			}
		}
		if !alias_map.is_empty() {
			sections.insert(format!("{}", "App Aliases".bright_magenta().bold()), alias_map);
		}

		// 3. global variables
		let mut vars_map = Map::new();
		if let Some(vars) = &self.vars {
			if vars.is_empty() {
				vars_map.insert("(no custom variables provided)".bright_red().to_string(), "".into());
			} else {
				for (name, target) in vars {
					vars_map.insert(format!("${name}").bright_red().to_string(), target.clone());
				}
			}
		}
		if !vars_map.is_empty() {
			sections.insert(format!("{}", "Global Variables".bright_red().bold()), vars_map);
		}

		// 4. global environment
		let mut env_map = Map::new();
		if let Some(env) = &self.env {
			if env.is_empty() {
				env_map.insert("(no environment overrides provided)".bright_blue().to_string(), "".into());
			} else {
				for (name, target) in env {
					env_map.insert(format!("${name}").bright_blue().to_string(), target.clone());
				}
			}
		}
		if !env_map.is_empty() {
			sections.insert(format!("{}", "Global Environment".bright_blue().bold()), env_map);
		}

		// generate rows and make box
		let rows = generate_rows(sections);
		make_table(f, "Config Info", rows)?;

		Ok(())
	}
}
