#![allow(unused)]

// --- imports ---
use std::fmt::{Formatter, Result};
use std::process::Command;
use console::measure_text_width;
use indexmap::IndexMap;
use regex::Regex;
use terminal_size::{Height, Width, terminal_size};
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{Err, LE};
use crate::launcher::Launcher;

// --- constants ---
pub const DEFAULT_CONFIG: &str = include_str!("../res/config.toml");
pub const DEFAULT_APP: &str = include_str!("../res/app.toml");

// --- functions ---
pub fn sanitize_app_name<S: Into<String>>(name: S) -> String {
	name.into().trim().replace(' ', "_").replace('\\', "/").trim_matches('/').to_string()
}

pub fn new_app(path: &PathBuf, name: String) -> Err<PathBuf> {
	let path = path.join(format!("apps/{}.toml", sanitize_app_name(name)));
	if path.exists() {
		return Err(LE::Other(format!(
			"file already exists: {}",
			path.display()
		)));
	}
	std::fs::write(&path, DEFAULT_APP)?;
	Ok(path)
}

pub fn new_config(path: &PathBuf) -> Err<()> {
	let path = path.join("config.toml");
	if path.exists() {
		return Ok(());
	}
	std::fs::write(path, DEFAULT_CONFIG)?;
	Ok(())
}

/// parse boolean from cli arg
pub fn parse_bool(s: &str) -> Option<bool> {
	match s.to_lowercase().trim() {
		"1" | "true" | "yes" | "y" | "on" => Some(true),
		"0" | "false" | "no" | "n" | "off" => Some(false),
		_ => None,
	}
}

pub fn app_resolver<'p>(
	launcher: &Launcher,
	query: &str,
	matches: Vec<&'p PathBuf>
) -> Err<&'p PathBuf> {
	// check if we are allowed to be interactive
	if !launcher.config.interactive || !atty::is(atty::Stream::Stdout) {
		return Err(LE::AmbiguousQuery(query.into(), matches.iter().map(|&p| p.to_string_lossy().into_owned()).collect()));
	}

	// interactive selection
	use dialoguer::{theme::ColorfulTheme, Select};

	let items: Vec<String> = matches
	.iter()
	.map(|p| p.to_string_lossy().to_string())
	.collect();

	let selection = Select::with_theme(&ColorfulTheme::default())
	.with_prompt("multiple apps found. please select one:")
	.items(&items)
	.default(0)
	.interact_opt()
	.map_err(LE::DialoguerError)?;

	match selection {
		Some(index) => Ok(matches[index]),
		None => Err(LE::Other("cancelled.".into())), // handle esc/ctrl+c
	}
}

/// puts child args in place of %! in parent args, or appends if no %! is found
pub fn sandwich_args(parent: Vec<String>, child: Vec<String>) -> Vec<String> {
	// find the index of the injection point
	if let Some(pos) = parent.iter().position(|arg| arg == "%!") {
		let mut final_args = Vec::new();

		// take everything BEFORE %!
		final_args.extend(parent[..pos].iter().cloned());

		// put the child args in the middle
		final_args.extend(child);

		// take everything AFTER %!
		final_args.extend(parent[pos + 1..].iter().cloned());

		final_args
	} else {
		// fallback if no %! is found: just append
		let mut fallback = parent;
		fallback.extend(child);
		fallback
	}
}

/// expands %var% to the value of var from config, or handles nested lookups like %apps.app_name.meta.name%, or leaves it unchanged if not found.
/// repeats up to 5 times to allow for nested variables.
pub fn expand_vars(text: &str, main: &Launcher) -> String {
	let re = Regex::new(r"%([^%]+)%").unwrap();
	let mut current_text = text.to_string();

	for _ in 0..5 {
		let new_text = re.replace_all(&current_text, |caps: &regex::Captures| {
			let full_key = &caps[1];
			let parts: Vec<&str> = full_key.split('.').collect();

			let resolved = match parts.as_slice() {
				// --- CONFIG SCOPE ---
				["config", "interactive"] => Some(main.config.interactive.to_string()),
									["config", "editor"] => main.config.editor.clone(),
									["config", "alias", k] => main.config.alias.get(*k).cloned(),
									["config", "vars", k] => main.config.vars.get(*k).cloned(),
									["config", "env", k] => main.config.env.get(*k).cloned(),

									// --- app scope ---
									// (currently disabled because i cant figure out a way to make it not try to expand itself)
									// Format: %apps.app_name.category.field%
									//["apps", app_query, category, field] => {
									//	main.load_app(app_query).ok().and_then(|app| {
									//		match *category {
									//			"meta" => match *field {
									//				"name" => app.meta.and_then(|m| m.name),
									//				"description" => app.meta.and_then(|m| m.description),
									//				"version" => app.meta.and_then(|m| m.version),
									//				_ => None,
									//			},
									//			"exec" => match *field {
									//				"bin" => Some(app.exec.bin),
									//				"args" => Some(app.exec.args.join(" ")), // Join args as string
									//				_ => None,
									//			},
									//			"env" => app.env.and_then(|e| e.get(*field).cloned()),
									//			_ => None,
									//		}
									//	})
									//}

									// --- fallback ---
									// if just %VAR%, check config.vars
									[k] => main.config.vars.get(*k).cloned(),

									_ => None,
			};

			resolved.unwrap_or_else(|| format!("%{}%", full_key))
		}).to_string();

		if new_text == current_text { break; }
		current_text = new_text;
	}
	current_text
}

#[cfg(unix)]
use std::os::unix::process::CommandExt;

/// opens {path} in the editor specified by config.editor, $EDITOR, $VISUAL, or a sane default if none of them are set.
pub fn open_in_editor(config: &Config, path: PathBuf) -> Err<()> {
	// determine the editor binary
	let editor = config.editor.clone()
		.or_else(|| std::env::var("EDITOR").ok())
		.or_else(|| std::env::var("VISUAL").ok())
		.unwrap_or_else(|| {
			if cfg!(windows) {
				"notepad".to_string()
			} else {
				"nano".to_string()
			}
		});

	let mut cmd = Command::new(editor);
	cmd.arg(path);

	// cross-platform "Replace Process" logic
	#[cfg(unix)]
	{
		// Hand over the TTY/Input/Output entirely to the editor
		let err = cmd.exec();
		return Err(LE::IoError(err));
	}

	#[cfg(windows)]
	{
		// Windows: Spawn and wait, then exit the launcher
		let mut child = cmd.spawn()?;
		child.wait()?;
		std::process::exit(0);
	}
}

pub fn edit_config(root_path: &PathBuf) -> Err<()> {
	let path = root_path.join("config.toml");
	if !path.exists() {
		return Err(LE::ConfigNotFound(path.to_string_lossy().into()));
	}
	let config = std::fs::read_to_string(&path)?;
	let config: Config = toml::from_str(&config)?;
	open_in_editor(&config, path)
}

/// wraps rows to fit terminal width. currently unused because of weird behavior with ANSI wrapped text. could be used later once fixed.
pub fn wrap_rows(rows: Vec<String>, term_w: usize) -> Vec<String> {
	let mut wrapped = Vec::new();

	for row in rows {
		let mut start = 0;
		let chars: Vec<char> = row.chars().collect(); // handle Unicode properly
		while start < measure_text_width(&row) {
			let end = (start + term_w).min(measure_text_width(&row));
			let slice: String = chars[start..end].iter().collect();
			wrapped.push(slice);
			start += term_w;
		}
	}

	wrapped
}

/// returns width of the current terminal, or 80 as a fallback if it can't be detected
pub fn get_term_width() -> usize {
	if let Some((Width(w), Height(_h))) = terminal_size() {
		w as usize
	} else {
		80 // fallback if we can't detect terminal
	}
}

/// creates a box with the given name and rows, fitting to terminal width. handles ANSI color codes properly.
pub fn make_box(f: &mut Formatter<'_>, name: &str, rows: Vec<String>) -> Result {
	let term_w = get_term_width();
	// wrap rows to fit terminal width minus borders
	// let rows = wrap_rows(rows, term_w - 4);

	// find the longest wrapped row
	let mut longest_row = rows.iter().map(|s| measure_text_width(s) + 2).max().unwrap_or(0);

	// top border
	let mut top = format!("╭─ {} ", name);
	let toplen = measure_text_width(&top);
	if longest_row > toplen {
		let remaining = longest_row.saturating_sub(toplen);
		top.push_str(&"─".repeat(remaining));
	} else {
		longest_row = toplen
	}
	top.push_str("─╮");
	writeln!(f, "{top}")?;

	// content rows
	for row in rows {
		let mut line = format!("│ {}", row);
		let remaining = longest_row.saturating_sub(measure_text_width(&line));
		line.push_str(&" ".repeat(remaining));
		line.push_str(" │");
		writeln!(f, "{line}")?;
	}

	// bottom border
	let mut bottom = "╰─".to_string();
	let remaining = longest_row.saturating_sub(measure_text_width(&bottom));
	bottom.push_str(&"─".repeat(remaining));
	bottom.push_str("─╯");
	writeln!(f, "{bottom}")?;

	Ok(())
}

/// generates rows for the config/app info display, handling ANSI color codes and wrapping values to fit terminal width (though currently ANSI breaks on line-wraps).
/// expects sections in the format of section name → (key → value).
pub fn generate_rows(
	sections: IndexMap<String, IndexMap<String, String>>,
) -> Vec<String> {
	let term_w = get_term_width().saturating_sub(4); // borders handled in make_box

	// find longest key length
	let (longest_key_length, _) = sections
		.iter()
		.flat_map(|(_, h)| h.iter())
		.map(|(k, v)| (measure_text_width(k), measure_text_width(v)))
		.fold((0, 0), |(mk, mv), (k, v)| (mk.max(k), mv.max(v)));

	let key_col_width = longest_key_length;
	let value_start_col = key_col_width + 5; // " : "
	let value_max_width = term_w.saturating_sub(value_start_col).max(1);

	let mut rows = Vec::new();

	for (section_name, section) in sections {
		// section header
		rows.push(format!("[ {} ]", section_name));

		for (k, v) in section {
			let key_pad = " ".repeat(key_col_width - measure_text_width(&k));
			let indent = " ".repeat(value_start_col);

			// if value is empty, just print the key without " : "
			if v.is_empty() {
				rows.push(format!("  {}{}", k, key_pad));
				continue;
			}

			let words: Vec<&str> = v.split_whitespace().collect();
			let mut current_line = String::new();
			let mut first_line = true;

			for word in words {
				let word_len = measure_text_width(word);

				// word longer than line → hard split
				if word_len > value_max_width {
					if !current_line.is_empty() {
						let line = if first_line {
							format!("  {}{} : {}", k, key_pad, current_line)
						} else {
							format!("{}{}", indent, current_line)
						};
						rows.push(line.chars().take(term_w).collect());
						current_line.clear();
						first_line = false;
					}

					let chars: Vec<char> = word.chars().collect();
					let mut start = 0;
					while start < chars.len() {
						let end = (start + value_max_width).min(chars.len());
						let slice: String = chars[start..end].iter().collect();

						let line = if first_line {
							format!("  {}{} : {}", k, key_pad, slice)
						} else {
							format!("{}{}", indent, slice)
						};
						rows.push(line.chars().take(term_w).collect());

						start = end;
						first_line = false;
					}

					continue;
				}

				let new_len = if current_line.is_empty() {
					word_len
				} else {
					measure_text_width(&current_line) + 1 + word_len
				};

				if new_len > value_max_width {
					let line = if first_line {
						format!("  {}{} : {}", k, key_pad, current_line)
					} else {
						format!("{}{}", indent, current_line)
					};
					rows.push(line.chars().take(term_w).collect());

					current_line.clear();
					first_line = false;
				}

				if !current_line.is_empty() {
					current_line.push(' ');
				}
				current_line.push_str(word);
			}

			// flush remainder
			if !current_line.is_empty() {
				let line = if first_line {
					format!("  {}{} : {}", k, key_pad, current_line)
				} else {
					format!("{}{}", indent, current_line)
				};
				rows.push(line.chars().take(term_w).collect());
			}
		}

		// empty row between sections
		rows.push(String::new());
	}

	rows
}
