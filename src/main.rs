// ran - run anything now
// a simple but customizable command-line launcher for games and programs.

// Copyright 2026 Hasibix Hasi

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at

//     http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// --- modules ---
mod cli;
mod app;
mod config;
mod launcher;
mod utils;
mod error;

// --- imports ---
use std::path::PathBuf;
use cli::*;
use colored::Colorize;
use error::{Err, LE};
use clap::Parser;
use terminal_size::{Width, terminal_size};

use crate::utils::{edit_config, new_app, new_config, open_in_editor, parse_bool, sanitize_app_name};
use crate::launcher::Launcher;

// --- functions ---
fn main() {
	if let Some(e) = real_main().err() {
		eprintln!("{}", e);
	}
}

/// cli handling
fn real_main() -> Err<()> {
	let cli = Cli::parse();
	let config_path = cli.config.unwrap_or(default_config_path()?);

	if !config_path.exists() {
		std::fs::create_dir_all(&config_path)?;
	}
	let config_file = config_path.join("config.toml");
	if !config_file.exists() {
		new_config(&config_path)?;
	}

	let m = Launcher::init(&config_path);
	let cmd = cli.command.ok_or(LE::NoCommandGiven)?;

	match cmd {
		Command::Launch(l) => m?.launch_app(&l.app, l.args, std::env::vars().collect(), l.background),
		Command::App(a) => {
			match a {
				AppCmd::Launch(l) => m?.launch_app(&l.app, l.args, std::env::vars().collect(), l.background),
				AppCmd::Info { app } => {
					match terminal_size() {
					    Some((Width(w), _)) if w >= 40 => {
					        let app = m?.load_app(&app)?;
					        println!("{}", app);
					        Ok(())
					    }
					    Some((Width(w), _)) => Err(LE::Other(
					        format!("terminal width ({}) too small to display app info (minimum 40 required)", w
					    ))),
					    None => Err(LE::Other(
					        "unable to determine terminal size".into(),
					    )),
					}
				}
				AppCmd::List => {
					println!("list of all specified applications");
					for (name, path) in &m?.apps {
						println!("{} {} {}", name.yellow(), "--".bright_black(), path.to_string_lossy().white())
					}
					Ok(())
				}
				AppCmd::Edit { app } => {
					let m = m?;
					open_in_editor(&m.config, m.find_app(&app)?.clone())
				}
				AppCmd::Print { app } => {
					let m = m?;
					let path = m.find_app(&app)?;
					println!("{}", std::fs::read_to_string(path)?);
					Ok(())
				}
				AppCmd::Create { app } => {
					let m = m?;
					open_in_editor(&m.config, new_app(&config_path, app)?.clone())
				},
				AppCmd::New { app } => new_app(&config_path, app).map(|_| ()),
				AppCmd::Delete { app, confirm } => {
					let launcher = m?;
					let path = config_path.join(format!("apps/{}.toml", sanitize_app_name(&app)));

					if !path.exists() {
						return Err(LE::Other(format!("app '{}' does not exist at {}", app, path.display())));
					}

					let delete = if confirm {
						true
					} else if launcher.config.interactive && atty::is(atty::Stream::Stdout) {
						use dialoguer::{theme::ColorfulTheme, Confirm};

						Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt(format!("are you sure you want to delete {}?", app))
							.default(false)
							.interact()
							.unwrap_or(false)
					} else {
						return Err(LE::Other("deletion requires confirmation. use -y/--yes or enable interactive mode in your config.".into()));
					};

					if delete {
						std::fs::remove_file(&path)
						.map_err(|e| LE::Other(format!("failed to delete file: {}", e)))?;
						println!("successfully deleted {}", path.display());
					} else {
						println!("deletion cancelled.");
					}

					Ok(())
				}
			}
		}
		Command::Config(c) => {
			match c {
				ConfigCmd::Edit => edit_config(&config_path),
				ConfigCmd::Print => {
					println!("{}", std::fs::read_to_string(config_path.join("config.toml"))?);
					Ok(())
				}
				ConfigCmd::Path => {
					println!("{}", config_path.to_string_lossy());
					Ok(())
				}
				ConfigCmd::Get { key } => {
					let launcher = m?;
					let c = &launcher.config;

					if let Some(key) = key {
						let parts: Vec<&str> = key.split('.').collect();

						let value: String = match parts.as_slice() {
							["interactive"] => c.interactive.to_string(),
							["editor"] => c.editor.clone().unwrap_or_else(|| "not specified".into()),

							["alias", k] => c.alias.get(*k)
							.cloned()
							.ok_or(LE::AliasNotFound(k.to_string()))?,

							["vars", k] => c.vars.get(*k)
							.cloned()
							.ok_or(LE::Other(format!("custom variable \"{k}\" is not defined")))? ,

							["env", k] => c.env.get(*k)
							.cloned()
							.ok_or(LE::Other(format!("environment variable \"{k}\" is not defined in config.env")))? ,

							_ => return Err(LE::Other(format!("invalid key \"{}\"", key))),
						};

						println!("{}", value);
					} else {
						print!("{}", c);
					}
					Ok(())
				}
				ConfigCmd::Set { key, value } => {
					let mut c = m?.config;
					let parts: Vec<&str> = key.split('.').collect();

					match parts.as_slice() {
						["interactive"] => c.interactive = parse_bool(&value).ok_or(LE::Other(format!("parse error: \"{value}\" is not a boolean")))?,
						["editor"] => c.editor = Some(value),

						["alias", k] => {
							c.alias.insert(k.to_string(), value.clone()).ok_or(LE::Other(format!("could not set alias.{k} to \"{value}\"")))?;
							()
						},

						["vars", k] => {
							c.vars.insert(k.to_string(), value.clone()).ok_or(LE::Other(format!("could not set vars.{k} to \"{value}\"")))?;
							()
						},

						["env", k] => {
							c.env.insert(k.to_string(), value.clone()).ok_or(LE::Other(format!("could not set env.{k} to \"{value}\"")))?;
							()
						},

						_ => return Err(LE::Other(format!("invalid key \"{}\"", key))),
					};

					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				ConfigCmd::Unset { key } => {
					let mut c = m?.config;
					let parts: Vec<&str> = key.split('.').collect();

					match parts.as_slice() {
						["interactive"] => c.interactive = atty::is(atty::Stream::Stdout),
						["editor"] => c.editor = None,

						["alias", k] => {
							c.alias.remove(*k);
						},

						["vars", k] => {
							c.vars.remove(*k);
						},

						["env", k] => {
							c.env.remove(*k);
						},

						_ => return Err(LE::Other(format!("invalid key \"{}\"", key))),
					};

					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				ConfigCmd::Info => {
					match terminal_size() {
					    Some((Width(w), _)) if w >= 40 => {
					        println!("{}", m?.config);
					        Ok(())
					    }
					    Some((Width(w), _)) => Err(LE::Other(
					        format!("terminal width ({}) too small to display config info (minimum 40 required)", w
					    ))),
					    None => Err(LE::Other(
					        "unable to determine terminal size".into(),
					    )),
					}
				}
			}
		}
		Command::Alias(a) => {
			match a {
				AliasCmd::Get { alias } => {
					println!("{}", m?.resolve_alias_chain(&alias)?.join(" -> ").trim());
					Ok(())
				}
				AliasCmd::Set { alias, value } => {
					let mut c = m?.config;
					c.alias.insert(alias, value);
					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				AliasCmd::Unset { alias } => {
					let mut c = m?.config;
					c.alias.remove(&alias);
					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				AliasCmd::List => {
					let m = m?;
					for (name, _) in &m.config.alias {
						let c = match m.resolve_alias_chain(&name).map(|c| c.join(" -> ").trim().to_string()) {
							Ok(s) => s,
							Err(e) => format!("{}", e),
						};
						println!("{c}");
					}
					Ok(())
				}
			}
		}
		Command::Var(a) => {
			match a {
				VarCmd::Get { key } => {
					println!("{}", m?.config.vars.get(&key).ok_or(LE::Other(format!("custom variable \"{key}\" is not defined")))?);
					Ok(())
				}
				VarCmd::Set { key, value } => {
					let mut c = m?.config;
					c.vars.insert(key, value);
					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				VarCmd::Unset { key } => {
					let mut c = m?.config;
					c.vars.remove(&key);
					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				VarCmd::List => {
					let m = m?;
					for (name, value) in &m.config.vars {
						println!("%{name}% = {value}");
					}
					Ok(())
				}
			}
		}
	}
}

/// determines the default config path based on XDG_CONFIG_HOME or HOME environment variables
fn default_config_path() -> Err<PathBuf> {
	let xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
	if let Some(c) = xdg_config {
		Ok(PathBuf::from(c).join("ran"))
	} else {
		Ok(PathBuf::from(std::env::var("HOME").map_err(|_| LE::Other("unable to find a suitable default config directory. ($HOME and $XDG_CONFIG_HOME are both invalid/unset)".into()))?).join("config/ran"))
	}
}
