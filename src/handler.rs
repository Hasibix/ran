// --- imports ---
use anyhow::{anyhow, bail, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::path::PathBuf;
use terminal_size::{terminal_size, Width};
use crate::app::{new_app, sanitize_app_name};
use crate::cli::*;
use crate::config::{new_config_file, Config};
use crate::launcher::Launcher;
use crate::resolver::Resolver;
use crate::util::fs::open_in_editor;

// --- definitions ---
pub struct CommandHandler {
	pub config_path: PathBuf,
}

// --- implementations ---
impl CommandHandler {
	pub fn new(config_path: PathBuf) -> Self {
		Self { config_path }
	}

	pub fn init_config(&self) -> Result<Config> {
		let config_file = self.config_path.join("config.toml");
		new_config_file(&config_file, false)?;
		Config::load(&config_file)
	}

	pub fn init_launcher(&self) -> Result<Launcher> {
		let config = self.init_config()?;
		Launcher::init(&self.config_path, config)
	}

	pub fn handle_command(&self, cmd: Command) -> Result<()> {
		match cmd {
			Command::Launch { name, args, background } => {
				self.handle_launch("launch", &name, args, background)?
			}
			Command::Cmd { cmd, name, args, background } => {
				self.handle_launch(&cmd, &name, args, background)?
			}

			Command::App(app_cmd) => self.handle_app_cmd(app_cmd)?,
			Command::Config(config_cmd) => self.handle_config_cmd(config_cmd)?,
			Command::Alias(alias_cmd) => self.handle_alias_cmd(alias_cmd)?,
			Command::Var(var_cmd) => self.handle_var_cmd(var_cmd)?,
		}

		Ok(())
	}

	// --- handlers ---
	// main
	fn handle_launch(&self, cmd: &str, query: &str, args: Vec<String>, background: bool) -> Result<()> {
		let l = self.init_launcher()?;
		l.launch_app(cmd, query, args, env::vars().collect(), background)
	}

	// others
	fn print_app(&self, app: &str, raw: bool) -> Result<()> {
		let l = self.init_launcher()?;
		match terminal_size() {
			Some((Width(w), _)) if !raw && w >= 40
			=> println!("{}", l.load_app(app)?),
			_ => println!("{}", fs::read_to_string(l.find_app(app)?)?),
		}
		Ok(())
	}
	fn print_config(&self, raw: bool) -> Result<()> {
		match terminal_size() {
			Some((Width(w), _)) if !raw && w >= 40 => println!("{}", self.init_config()?),
			_ => println!("{}", fs::read_to_string(self.config_path.join("config.toml"))?),
		}
		Ok(())
	}

	fn handle_app_cmd(&self, cmd: AppCmd) -> Result<()> {
		match cmd {
			AppCmd::List => {
				let l = self.init_launcher()?;
				println!("list of all specified applications");
				for (name, path) in &l.apps {
					println!(
						"{} {} {}",
						name.yellow(),
						"--".bright_black(),
						path.to_string_lossy().white()
					)
				}
			}
			AppCmd::Edit { app } => open_in_editor(self.init_launcher()?.find_app(&app)?, true)?,
			AppCmd::Print { app, raw } => self.print_app(&app, raw)?,

			AppCmd::Get { app, key, raw } => if let Some(key) = key {
				println!(
					"{}",
					self.init_launcher()?
						.load_app(&app)?
						.get(&key).ok_or(anyhow!("invalid key '{key}'"))?
				);
			} else {
				self.print_app(&app, raw)?;
			}
			AppCmd::Set { app, key, value } => {
				let l = self.init_launcher()?;
				let app_file = l.find_app(&app)?;
				let mut app = l.load_app_from(app_file)?;
				app.set(&key, value)?;
				app.save(app_file)?;
			}
			AppCmd::Unset { app, key } => {
				let l = self.init_launcher()?;
				let app_file = l.find_app(&app)?;
				let mut app = l.load_app_from(app_file)?;
				app.unset(&key)?;
				app.save(app_file)?;
			}

			AppCmd::Create { app, clean, edit } => {
				let app_file = new_app(&self.config_path, app, clean)?;
				if edit {
					open_in_editor(&app_file, true)?;
				}
			}
			AppCmd::Delete { app, yes } => {
				let l = self.init_launcher()?;
				let path = self.config_path.join(format!("apps/{}.toml", sanitize_app_name(&app)));

				if !path.exists() {
					bail!("app '{app}' does not exist at '{}'", path.display());
				}

				let delete = if yes {
					true
				} else if !l.config.noninteractive && atty::is(atty::Stream::Stdout) {
					use dialoguer::{theme::ColorfulTheme, Confirm};

					Confirm::with_theme(&ColorfulTheme::default())
						.with_prompt(format!("are you sure you want to delete {app}?"))
						.default(false)
						.interact()
						.unwrap_or(false)
				} else {
					bail!(
						"deletion requires confirmation. use -y/--yes or enable interactive mode in your config."
					);
				};

				if delete {
					fs::remove_file(&path)
					.map_err(|e| anyhow!("failed to delete file: {e}"))?;
					println!("successfully deleted {}", path.display());
				} else {
					println!("deletion cancelled.");
				}
			}
		}
		Ok(())
	}
	fn handle_config_cmd(&self, cmd: ConfigCmd) -> Result<()> {
		match cmd {
			ConfigCmd::Path => println!("{}", self.config_path.display()),
			ConfigCmd::Edit => open_in_editor(&self.config_path.join("config.toml"), true)?,
			ConfigCmd::Print { raw } => self.print_config(raw)?,

			ConfigCmd::Get { key, raw } => if let Some(key) = key {
				println!(
					"{}",
					self.init_config()?
						.get(&key)
						.ok_or(anyhow!("invalid key '{key}'"))?
				);
			} else {
				self.print_config(raw)?;
			}
			ConfigCmd::Set { key, value } => {
				let mut c = self.init_config()?;
				c.set(&key, value)?;
				c.save(&self.config_path.join("config.toml"))?;
			}
			ConfigCmd::Unset { key } => {
				let mut c = self.init_config()?;
				c.unset(&key)?;
				c.save(&self.config_path.join("config.toml"))?;
			}

			ConfigCmd::Init { yes, clean, edit } => {
				let config_file = self.config_path.join("config.toml");
				if config_file.exists() {
					println!("a config file already exist in '{}'", config_file.display());
					let c = self.init_config()?;
					let delete = if yes {
						true
					} else if !c.noninteractive && atty::is(atty::Stream::Stdout) {
						use dialoguer::{theme::ColorfulTheme, Confirm};

						Confirm::with_theme(&ColorfulTheme::default())
							.with_prompt(format!("do you want to delete the existing config and reinitialize?"))
							.default(false)
							.interact()
							.unwrap_or(false)
					} else {
						bail!(
							"deletion requires confirmation. use -y/--yes or enable interactive mode in your config."
						);
					};
					if delete {
						fs::remove_file(&config_file)
						.map_err(|e| anyhow!("failed to delete file: {e}"))?;
						println!("successfully deleted {}", config_file.display());
					} else {
						println!("re-initialization cancelled.");
					}
				}
				new_config_file(&config_file, clean)?;
				println!("initalized config file in '{}'", config_file.display());
				if edit {
					open_in_editor(&config_file, true)?;
				}
			}
		}
		Ok(())
	}
	fn handle_alias_cmd(&self, cmd: AliasCmd) -> Result<()> {
		match cmd {
			AliasCmd::List => {
				let l = self.init_launcher()?;
				let resolver = Resolver::new(&l);
				if let Some(alias) = &l.config.alias {
					println!("list of all specified app aliases");
					for name in alias.keys() {
						let chain_result = resolver.resolve_alias_chain(&name.trim());
						let pretty_chain = match chain_result {
							Ok(chain) => {
								// if there’s no chain, just print the key
								if chain.is_empty() {
									name.bright_magenta().bold().to_string()
								} else {
									let len = chain.len();
									chain
										.into_iter()
										.enumerate()
										.map(|(i, item)| {
											if i == 0 {
												// first in chain gets bold magenta
												item.bright_magenta().bold().to_string()
											} else if i == len - 1 {
												// last in chain gets yellow
												item.bright_yellow().to_string()
											} else {
												item.bright_magenta().bold().to_string()
											}
										})
										.collect::<Vec<_>>()
										.join(&" -> ".bright_black().to_string())
								}
							}
							Err(e) => {
								format!(
									"{} -> {e}",
									name.bright_magenta().bold().to_string()
								).to_string()
							}
						};
						println!("{pretty_chain}");
					}
				} else {
					println!("no app aliases were defined");
				}
			}

			AliasCmd::Get { key, unresolved } => if !unresolved {
				let l = self.init_launcher()?;
				let resolver = Resolver::new(&l);
				let chain_result = resolver.resolve_alias_chain(&key);
				let pretty_chain = match chain_result {
					Ok(chain) => {
						if chain.is_empty() {
							bail!("undefined app alias '{key}'");
						} else {
							let len = chain.len();
							chain
								.into_iter()
								.enumerate()
								.map(|(i, item)| {
									if i == 0 {
										// first in chain gets bold magenta
										item.bright_magenta().bold().to_string()
									} else if i == len - 1 {
										// last in chain gets yellow
										item.bright_yellow().to_string()
									} else {
										item.bright_magenta().bold().to_string()
									}
								})
								.collect::<Vec<_>>()
								.join(&(" -> ".bright_black().to_string()))
						}
					}
					Err(e) => {
						format!(
							"{} -> {e}",
							&(key.bright_magenta().bold().to_string())
						)
					}
				};
				println!("{pretty_chain}");
			} else {
				println!(
					"{}",
					self.init_config()?
						.get(&format!("alias.{key}"))
						.ok_or(anyhow!("undefined app alias '{key}'"))?
				);
			}
			AliasCmd::Set { key, value } => {
				let mut c = self.init_config()?;
				c.set(&format!("alias.{key}"), value)?;
				c.save(&self.config_path.join("config.toml"))?;
			}
			AliasCmd::Unset { key } => {
				let mut c = self.init_config()?;
				c.unset(&format!("alias.{key}"))?;
				c.save(&self.config_path.join("config.toml"))?;
			}
		}
		Ok(())
	}
	fn handle_var_cmd(&self, cmd: VarCmd) -> Result<()> {
		match cmd {
			VarCmd::List => {
				let c = self.init_config()?;
				if let Some(vars) = &c.vars {
					println!("list of all specified global variables");
					for (key, value) in vars {
						println!(
							"{} {} {value}",
							format!("${key}").bright_red(),
							format!("=").bright_black()
						);
					}
				} else {
					println!("no app aliases were defined");
				}
			}

			VarCmd::Get { key } => println!(
				"{}",
				self.init_config()?
					.get(&format!("vars.{key}"))
					.ok_or(anyhow!("undefined variable '{key}'"))?
			),
			VarCmd::Set { key, value } => {
				let mut c = self.init_config()?;
				c.set(&format!("vars.{key}"), value)?;
				c.save(&self.config_path.join("config.toml"))?;
			}
			VarCmd::Unset { key } => {
				let mut c = self.init_config()?;
				c.unset(&format!("vars.{key}"))?;
				c.save(&self.config_path.join("config.toml"))?;
			}
		}
		Ok(())
	}
}
