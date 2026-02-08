// Imports

use clap::{Parser, Subcommand};
use std::path::PathBuf;

// Definitions

/// RAN - Run Anything Now
///
/// A simple but highly customizable command-line app for games and programs.
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
		help = "Defaults to $XDG_CONFIG_HOME/ran or $HOME/.config/ran",
		long_help = "Path for config files (e.g. general config or app list)",
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

/// Launch an application (with arguments, if needed)
#[derive(Parser)]
pub struct LaunchCmd {
	/// Application to be launched
	pub app: String,
	/// Arguments (redirected to application, used if needed)
	pub args: Vec<String>,
	/// Force launch (fails fast on issues)
	#[arg(short, long)]
	pub force: bool,
	/// Run the app in background (in a new terminal session)
	#[arg(short, long)]
	pub background: bool,
}

/// Application management
#[derive(Subcommand)]
pub enum AppCmd {
	Launch(LaunchCmd),
	/// Pretty-prints all information about an application based on definition
	Info {
		app: String,
	},
	/// List all applications
	List,
	/// Edit the specified application's definition file in the current config directory (using $VISUAL or $EDITOR. Falls back to `nano` or `notepad` if they're not set)
	Edit {
		app: String
	},
	/// Print the specified application's raw definition file in the current config directory (TOML)
	Print {
		app: String
	},
	/// Creates a dummy application file for your application name opens it with your text editor.
	Create {
		app: String
	},
	/// Creates a dummy application
	New {
		app: String
	},
	/// Deletes an application definition file (only definition (TOML) file, not the installed application)
	Delete {
		app: String,
		#[arg(short='y', long="yes")]
		confirm: bool
	},
}

/// Global configuration management
#[derive(Subcommand)]
pub enum ConfigCmd {
	/// Edit the current global configuration file in current config directory (using $VISUAL or $EDITOR. Falls back to `nano` if they're not set)
	Edit,
	/// Print the raw global configuration file in current config directory (TOML)
	Print,
	/// Prints the path currently being used as the config path
	Path,
	/// Prints current config or the specified section or key
	Get {
		key: Option<String>
	},
	/// Set config
	Set {
		key: String,
		value: String,
	},
	/// Unsets a config value
	Unset {
		key: String
	},
	// Pretty-prints the entire configuration data
	List,
}

/// Alias management
#[derive(Subcommand)]
pub enum AliasCmd {
	/// Get the value of an alias
	Get {
		alias: String
	},
	/// Set an alias
	Set {
		alias: String,
		value: String
	},
	/// Unset (remove) an alias
	Unset {
		alias: String
	},
	/// List all aliases
	List,
}

/// Global variables management
#[derive(Subcommand)]
pub enum VarCmd {
	/// Get the value of a variable
	Get {
		key: String
	},
	/// Set a variable (accessible via %config.vars.KEY%)
	Set {
		key: String,
		value: String
	},
	/// Remove a variable
	Unset {
		key: String
	},
	/// List all custom variables
	List,
}
