mod cli;
mod app;
mod config;
mod launcher;
mod helpers;
mod error;

use std::path::PathBuf;

use cli::*;
use colored::Colorize;
use error::{Err, LE};
use clap::Parser;

use crate::{helpers::{edit_config, new_app, new_config, open_in_editor, parse_bool, sanitize_app_name}, launcher::Launcher};

fn main() {
	if let Some(e) = real_main().err() {
		eprintln!("{}", e);
	}
}


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
					let app = m?.load_app(&app)?;
					println!("{}", app);
					Ok(())
				}
				AppCmd::List => {
					println!("List of all specified applications");
					for (name, path) in &m?.apps {
						println!("{} {} {}", name.yellow(), "--".bright_black(), path.to_string_lossy().white())
					}
					Ok(())
				}
				AppCmd::Edit { app } => open_in_editor(m?.find_app(&app)?.clone()),
				AppCmd::Print { app } => {
					let m = m?;
					let path = m.find_app(&app)?;
					println!("{}", std::fs::read_to_string(path)?);
					Ok(())
				}
				AppCmd::Create { app } => open_in_editor(new_app(&config_path, app)?),
				AppCmd::New { app } => new_app(&config_path, app).map(|_| ()),
				AppCmd::Delete { app, confirm } => {
					let launcher = m?;
					let path = config_path.join(format!("apps/{}.toml", sanitize_app_name(&app)));

					if !path.exists() {
						return Err(LE::Other(format!("App '{}' does not exist at {}", app, path.display())));
					}

					let delete = if confirm {
						true
					} else if launcher.config.interactive && atty::is(atty::Stream::Stdout) {
						use dialoguer::{theme::ColorfulTheme, Confirm};

						Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt(format!("Are you sure you want to delete {}?", app))
							.default(false)
							.interact()
							.unwrap_or(false)
					} else {
						return Err(LE::Other("Deletion requires confirmation. Use -y/--yes or enable interactive mode in your config.".into()));
					};

					if delete {
						std::fs::remove_file(&path)
						.map_err(|e| LE::Other(format!("Failed to delete file: {}", e)))?;
						println!("Successfully deleted {}", path.display());
					} else {
						println!("Deletion cancelled.");
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
							["terminal_runner"] => c.terminal_runner.clone(),

							["alias", k] => c.alias.get(*k)
							.cloned()
							.ok_or(LE::AliasNotFound(k.to_string()))?,

							["vars", k] => c.vars.get(*k)
							.cloned()
							.ok_or(LE::Other(format!("Global variable \"{k}\" not defined")))? ,

							["env", k] => c.env.get(*k)
							.cloned()
							.ok_or(LE::Other(format!("Env variable \"{k}\" not defined in config.env")))? ,

							_ => return Err(LE::Other(format!("Invalid key \"{}\"", key))),
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
						["interactive"] => c.interactive = parse_bool(&value).ok_or(LE::Other(format!("Parse error: \"{value}\" is not a boolean")))?,
						["terminal_runner"] => c.terminal_runner = value,

						["alias", k] => {
							c.alias.insert(k.to_string(), value.clone()).ok_or(LE::Other(format!("Could not set alias.{k} to \"{value}\"")))?;
							()
						},

						["vars", k] => {
							c.vars.insert(k.to_string(), value.clone()).ok_or(LE::Other(format!("Could not set vars.{k} to \"{value}\"")))?;
							()
						},

						["env", k] => {
							c.env.insert(k.to_string(), value.clone()).ok_or(LE::Other(format!("Could not set env.{k} to \"{value}\"")))?;
							()
						},

						_ => return Err(LE::Other(format!("Invalid key \"{}\"", key))),
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
						["terminal_runner"] => c.terminal_runner = "sh %! &".into(),

						["alias", k] => {
							c.alias.remove(*k);
						},

						["vars", k] => {
							c.vars.remove(*k);
						},

						["env", k] => {
							c.env.remove(*k);
						},

						_ => return Err(LE::Other(format!("Invalid key \"{}\"", key))),
					};

					let path = config_path.join("config.toml");
					std::fs::write(path, toml::to_string_pretty(&c)?)?;
					Ok(())
				}
				ConfigCmd::List => {
					println!("{}", m?.config);
					Ok(())
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
					println!("{}", m?.config.vars.get(&key).ok_or(LE::Other(format!("Global variable \"{key}\" not defined")))?);
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

fn default_config_path() -> Err<PathBuf> {
	let xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
	if let Some(c) = xdg_config {
		Ok(PathBuf::from(c).join("ran"))
	} else {
		Ok(PathBuf::from(std::env::var("HOME").map_err(|_| LE::Other("Unable to find a suitable default config directory. ($HOME and $XDG_CONFIG_HOME are both invalid/unset)".into()))?).join("config/ran"))
	}
}
