use std::process::Command;
use regex::Regex;
use std::path::PathBuf;

use crate::error::{Err, LE};
use crate::launcher::Launcher;

pub const DEFAULT_CONFIG: &str = include_str!("../res/config.toml");
pub const DEFAULT_APP: &str = include_str!("../res/app.toml");

pub fn sanitize_app_name<S: Into<String>>(name: S) -> String {
	name.into().trim().replace(' ', "_").replace('\\', "/").trim_matches('/').to_string()
}

pub fn new_app(path: &PathBuf, name: String) -> Err<PathBuf> {
	let path = path.join(format!("apps/{}.toml", sanitize_app_name(name)));

	// Check if the file already exists to prevent accidental overwrites
	if path.exists() {
		return Err(LE::Other(format!(
			"File already exists: {}",
			path.display()
		)));
	}

	// Write the embedded DEFAULT_APP string to the new path
	std::fs::write(&path, DEFAULT_APP)?;

	Ok(path)
}

pub fn new_config(path: &PathBuf) -> Err<()> {
	let path = path.join("config.toml");

	// Check if the file already exists to prevent accidental overwrites
	if path.exists() {
		return Ok(());
	}

	// Write the embedded DEFAULT_CONFIG string to the new path
	std::fs::write(path, DEFAULT_CONFIG)?;

	Ok(())
}

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
	// 1. Check if we are allowed to be interactive
	if !launcher.config.interactive || !atty::is(atty::Stream::Stdout) {
		return Err(LE::AmbiguousQuery(query.into(), matches.iter().map(|&p| p.to_string_lossy().into_owned()).collect()));
	}

	// 3. Interactive Selection
	use dialoguer::{theme::ColorfulTheme, Select};

	let items: Vec<String> = matches
	.iter()
	.map(|p| p.to_string_lossy().to_string())
	.collect();

	let selection = Select::with_theme(&ColorfulTheme::default())
	.with_prompt("Multiple apps found. Please select one:")
	.items(&items)
	.default(0)
	.interact_opt()
	.map_err(LE::DialoguerError)?;

	match selection {
		Some(index) => Ok(matches[index]),
		None => Err(LE::Other("Cancelled.".into())), // Handle Esc/Ctrl+C
	}
}

pub fn sandwich_args(parent: Vec<String>, child: Vec<String>) -> Vec<String> {
	// 1. Find the index of the injection point
	if let Some(pos) = parent.iter().position(|arg| arg == "%!") {
		let mut final_args = Vec::new();

		// 2. Take everything BEFORE %!
		final_args.extend(parent[..pos].iter().cloned());

		// 3. Put the child args in the middle
		final_args.extend(child);

		// 4. Take everything AFTER %!
		final_args.extend(parent[pos + 1..].iter().cloned());

		final_args
	} else {
		// Fallback if no %! is found: just append
		let mut fallback = parent;
		fallback.extend(child);
		fallback
	}
}

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
									  ["config", "terminal_runner"] => Some(main.config.terminal_runner.clone()),
									  ["config", "alias", k] => main.config.alias.get(*k).cloned(),
									  ["config", "vars", k] => main.config.vars.get(*k).cloned(),
									  ["config", "env", k] => main.config.env.get(*k).cloned(),

									  // --- APPS SCOPE ---
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

									  // --- FALLBACK ---
									  // If just %VAR%, check config.variables first
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

pub fn open_in_editor(path: PathBuf) -> Err<()> {
	// 1. Determine the editor binary
	let editor = if cfg!(windows) {
		std::env::var("EDITOR")
		.or_else(|_| std::env::var("VISUAL"))
		.unwrap_or_else(|_| "notepad".to_string())
	} else {
		std::env::var("EDITOR")
		.or_else(|_| std::env::var("VISUAL"))
		.unwrap_or_else(|_| "nano".to_string()) // Sane Linux default
	};

	let mut cmd = Command::new(editor);
	cmd.arg(path);

	// 2. Cross-platform "Replace Process" logic
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
	open_in_editor(path)
}
