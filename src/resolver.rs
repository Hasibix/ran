// --- imports ---
use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap as Map;
use crate::app::App;
use crate::launcher::Launcher;
use crate::util::args::sandwich_args;

// --- definitions ---
pub struct ResolvedParts {
	pub bin: String,
	pub args: Vec<String>,
	pub env: Map<String, String>,
}

pub struct Resolver<'a> {
	pub launcher: &'a Launcher,
}

// --- implementations ---
impl<'a> Resolver<'a> {
	pub fn new(launcher: &'a Launcher) -> Self {
		Self { launcher }
	}

	/// resolves the commands executable, arguments, and environment variables
	/// supports nested runners (bin starting with '@')
	pub fn resolve_command(&self, app: &App, command: &str) -> Result<ResolvedParts> {
		let cmd = app.cmds.get(command)
			.ok_or(anyhow!("command not found: '{command}'"))?;

		let mut res_parts = if cmd.bin.starts_with('@') {
			// nested runner
			let full = &cmd.bin[1..];
			if full.is_empty() { bail!("runner app name cannot be empty!"); }

			let parts: Vec<&str> = full.split_whitespace().collect();
			let (runner_name, sub_command) = match parts.as_slice() {
				[runner] => (*runner, "launch"),
				[runner, command] => (*runner, *command),
				_ => bail!("invalid runner '{}'", cmd.bin),
			};

			let runner_app = self.launcher.load_app(runner_name)?;
			self.resolve_command(&runner_app, sub_command)?
		} else {
			// base case
			ResolvedParts {
				bin: cmd.bin.clone(),
				args: Vec::new(),
				env: Map::new(),
			}
		};

		// merge arguments
		res_parts.args = if !res_parts.args.is_empty() {
			sandwich_args(res_parts.args, cmd.args.clone())
		} else {
			cmd.args.clone()
		};

		// merge environment
		if let Some(e) = &app.env {
			res_parts.env.extend(e.clone());
		}
		if let Some(e) = &cmd.env {
			res_parts.env.extend(e.clone());
		}

		Ok(res_parts)
	}

	/// recursively resolves a variable by key with infinite-loop detection
	pub fn resolve_variable(&self, app: Option<&App>, key: &str, stack: &mut Vec<String>) -> Result<Option<String>> {
		if stack.contains(&key.to_string()) {
			return Err(anyhow!(
				"recursive variable reference detected: {} -> {}",
				stack.join(" -> "),
				key
			));
		}

		stack.push(key.to_string());
		let parts: Vec<&str> = key.split('.').collect();

		let value = match parts.as_slice() {
			["config", rest @ ..] => self.launcher.config.get_slice(rest),
			["apps", app_name, rest @ ..] => {
				self.launcher.load_app(*app_name).ok()
					.and_then(|a| a.get_slice(rest))
			}
			["self", rest @ ..] => app.and_then(|a| a.get_slice(rest)),
			[k] => {
				let resolved = app.and_then(|a| {
					a.vars.as_ref().and_then(|vars| vars.get(*k).cloned())
				});
				resolved.or_else(|| self.launcher.config.vars.as_ref().and_then(|vars| vars.get(*k).cloned()))
			}
			_ => None,
		};

		let expanded = if let Some(val) = value {
			self.expand_string(app, &val, stack)?
		} else { None };

		stack.pop();
		Ok(expanded)
	}

	/// expands variables in a string with nested `${...}` and single-word `$NAME`
	pub fn expand_string(&self, app: Option<&App>, text: &str, stack: &mut Vec<String>) -> Result<Option<String>> {
		let mut result = String::with_capacity(text.len());
		let mut chars = text.chars().peekable();

		while let Some(c) = chars.next() {
			if c == '$' {
				if chars.peek() == Some(&'$') {
					// escaped $, skip next $
					chars.next();
					result.push('$');
				} else if chars.peek() == Some(&'{') {
					// nested ${...}
					chars.next(); // skip '{'
					let mut inner = String::new();
					let mut brace_level = 1;
					while let Some(ch) = chars.next() {
						match ch {
							'{' => {
								inner.push(ch);
								brace_level += 1;
							}
							'}' => {
								brace_level -= 1;
								if brace_level == 0 { break; }
								inner.push(ch);
							}
							_ => inner.push(ch),
						}
					}
					let val = self.resolve_variable(app, &inner, stack)?.unwrap_or(format!("${{{}}}", inner));
					result.push_str(&val);
				} else if let Some(&next_c) = chars.peek() {
					// single-word $NAME
					if next_c.is_ascii_alphabetic() || next_c == '_' {
						let mut name = String::new();
						while let Some(&ch) = chars.peek() {
							if ch.is_ascii_alphanumeric() || ch == '_' {
								name.push(ch);
								chars.next();
							} else { break; }
						}
						let val = self.resolve_variable(app, &name, stack)?.unwrap_or(format!("${}", name));
						result.push_str(&val);
					} else {
						result.push('$');
					}
				} else {
					result.push('$');
				}
			} else {
				result.push(c);
			}
		}

		Ok(Some(result))
	}

	/// public API to expand variables, initializes recursion stack
	pub fn expand(&self, app: Option<&App>, text: &str) -> Result<String> {
		Ok(self.expand_string(app, text, &mut Vec::new())?.unwrap_or_default())
	}

	/// resolves alias chain with infinite-loop detection
	pub fn resolve_alias_chain(&self, start_key: &str) -> Result<Vec<String>> {
		let alias = self.launcher.config.alias
			.as_ref()
			.ok_or(anyhow!("alias map was not defined!"))?;

		let mut chain = vec![start_key.to_string()];
		let mut current = start_key;

		while let Some(next) = alias.get(current) {
			if chain.contains(next) {
				chain.push(next.to_string());
				bail!("infinite recursion in alias expansion: {}", chain.join(" -> "));
			}
			chain.push(next.to_string());
			current = next;
		}

		if chain.len() < 2 {
			bail!("invalid alias '{start_key}'")
		}

		Ok(chain)
	}
}
