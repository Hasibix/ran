// --- imports ---
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// --- definitions ---
/// ran - run anything now
///
/// a simple but customizable command-line launcher for games and programs.
/// 
/// copyright (c) 2026 Hasibix Hasi.
/// licensed under Apache 2.0.
#[derive(Parser)]
#[command(
	author,
	version,
	about,
	long_about,
	arg_required_else_help = true
)]
pub struct Cli {
	#[arg(
		long,
		env = "RANCFG",
		help = "defaults to $XDG_CONFIG_HOME/ran or $HOME/.config/ran",
		long_help = "path for config files (e.g. general config or app list)",
	)]
	pub config: Option<PathBuf>,

	#[command(subcommand)]
	pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
	Launch(LaunchCmd),
	#[command(subcommand)]
	App(AppCmd),
	#[command(subcommand)]
	Config(ConfigCmd),
	#[command(subcommand)]
	Alias(AliasCmd),
	#[command(subcommand)]
	Var(VarCmd),
}

/// launch an application (with arguments, if needed)
#[derive(Parser)]
pub struct LaunchCmd {
	/// application to be launched
	pub app: String,
	/// arguments (redirected to application, used if needed)
	pub args: Vec<String>,
	/// force launch (fails fast on issues)
	#[arg(short, long)]
	pub force: bool,
	/// run the app in background (in a new terminal session)
	#[arg(short, long)]
	pub background: bool,
}

/// application management
#[derive(Subcommand)]
pub enum AppCmd {
	Launch(LaunchCmd),
	/// pretty-prints all information about an application based on definition
	Info {
		app: String,
	},
	/// lists all applications (defined in config_path/apps/)
	List,
	/// opens the specified application's definition file using your preferred text editor. (config.editor or $VISUAL or $EDITOR, falls back to `nano`/`notepad` if none are set)
	Edit {
		app: String
	},
	/// prints the specified application's raw definition file (TOML)
	Print {
		app: String
	},
	/// creates a dummy application file for your application name opens it with your text editor.
	/// treats {app} as the full name of the app
	Create {
		app: String
	},
	/// creates a dummy application
	/// treats {app} as the full name of the app
	New {
		app: String
	},
	/// deletes an application definition file (only definition (TOML) file, not the installed application).
	/// requires full path to the app (e.g. "games/silksong")
	Delete {
		app: String,
		#[arg(short='y', long="yes")]
		confirm: bool
	},
}

/// global configuration management
#[derive(Subcommand)]
pub enum ConfigCmd {
	/// opens the current global configuration file using your preferred text editor. (config.editor or $VISUAL or $EDITOR, falls back to `nano`/`notepad` if none are set)
	Edit,
	/// prints the raw global configuration file (TOML)
	Print,
	/// prints the path currently being used as the config path (aka where config.toml and apps/ are located)
	Path,
	/// prints current config or the specified key
	Get {
		key: Option<String>
	},
	/// sets a config value (e.g. "editor" or "env.KEY")
	Set {
		key: String,
		value: String,
	},
	/// unsets a config value
	Unset {
		key: String
	},
	/// pretty-prints the entire configuration data
	Info,
}

/// alias management
#[derive(Subcommand)]
pub enum AliasCmd {
	/// prints the value of an alias
	Get {
		alias: String
	},
	/// sets an app alias
	Set {
		alias: String,
		value: String
	},
	/// unsets (remove) an app alias
	Unset {
		alias: String
	},
	/// lists all app aliases
	List,
}

/// global variables management
#[derive(Subcommand)]
pub enum VarCmd {
	/// prints the value of a variable
	Get {
		key: String
	},
	/// sets a variable (accessible via %config.vars.key% or %key%)
	Set {
		key: String,
		value: String
	},
	/// removes a variable
	Unset {
		key: String
	},
	/// lists all custom variables
	List,
}
