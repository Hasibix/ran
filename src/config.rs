// Imports

use serde::{Serialize, Deserialize};
use std::default::Default;
use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result};
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::*;

// Definitions

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
	#[serde(default = "default_interactive")]
	pub interactive: bool,
	#[serde(default = "default_terminal")]
	pub terminal_runner: String,

	#[serde(default)]
	pub alias: HashMap<String, String>,

	#[serde(default)]
	pub vars: HashMap<String, String>,

	#[serde(default)]
	pub env: HashMap<String, String>,
}

// Implementations

impl Display for Config {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		let mut table = Table::new();
		table
		.load_preset(UTF8_FULL_CONDENSED)
		.apply_modifier(UTF8_ROUND_CORNERS)
		.set_content_arrangement(ContentArrangement::Dynamic);

		// --- SECTION: General Settings ---
		table.add_row(vec![
			Cell::new("General Settings").add_attribute(Attribute::Bold).fg(Color::Cyan),
					  Cell::new("Value").add_attribute(Attribute::Bold).fg(Color::Cyan),
		]);
		table.add_row(vec!["Interactive Mode", &self.interactive.to_string()]);
		table.add_row(vec!["Terminal Runner", &self.terminal_runner]);

		// --- SECTION: Aliases ---
		if !self.alias.is_empty() {
			table.add_row(vec![
				Cell::new("\nApp Aliases").add_attribute(Attribute::Bold).fg(Color::Magenta),
						  Cell::new("\nTarget"),
			]);
			for (name, value) in &self.alias {
				table.add_row(vec![name, value]);
			}
		}

		// --- SECTION: Global Variables ---
		if !self.vars.is_empty() {
			table.add_row(vec![
				Cell::new("\nGlobal Variables").add_attribute(Attribute::Bold).fg(Color::Yellow),
						  Cell::new("\nValue"),
			]);
			for (name, value) in &self.vars {
				table.add_row(vec![format!("%{}%", name), value.to_string()]);
			}
		}

		// --- SECTION: Environment ---
		if !self.env.is_empty() {
			table.add_row(vec![
				Cell::new("\nGlobal Environment").add_attribute(Attribute::Bold).fg(Color::Blue),
						  Cell::new("\nValue"),
			]);
			for (name, value) in &self.env {
				table.add_row(vec![format!("${}", name), value.to_string()]);
			}
		}

		write!(f, "{}", table)
	}
}

// Functions

pub fn default_interactive() -> bool { true }

pub fn default_terminal() -> String {
	"alacritty -e %!".to_string()
}
