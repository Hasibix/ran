#![allow(dead_code)]

// Imports

use std::fmt;

// Definitions

pub type LE = LauncherError;
pub type Err<T> = Result<T, LauncherError>;

#[derive(Debug)]
pub enum LauncherError {
	InvalidAlias(String, String),
	InvalidConfig(Option<String>),
	InvalidApp(String, Option<String>),
	ConfigNotFound(String),
	AppNotFound(String),
	AliasNotFound(String),
	CircularAlias(Vec<String>),
	IoError(std::io::Error),
	DialoguerError(dialoguer::Error),
	ParseError(toml::de::Error),
	SerializationError(toml::ser::Error),
	AmbiguousQuery(String, Vec<String>),
	NoCommandGiven,
	Other(String),
}

// Implementations

impl fmt::Display for LauncherError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::InvalidAlias(alias, target) => write!(f, "Invalid alias {alias} -> {target}"),
			Self::InvalidConfig(reason) => {
				write!(f, "Invalid config")?;
				if let Some(r) = reason {
					write!(f, " -- {r}")?;
				}
				Ok(())
			}
			Self::InvalidApp(path, reason) => {
				write!(f, "Invalid app definition in {path}")?;
				if let Some(r) = reason {
					write!(f, " -- {r}")?;
				}
				Ok(())
			}
			Self::ConfigNotFound(path) => write!(f, "Config file not found in {path}"),
			Self::AppNotFound(name) => write!(f, "App definition not found for {name}"),
			Self::AliasNotFound(name) => write!(f, "Alias \"{name}\" was not found"),
			Self::CircularAlias(c) => write!(f, "Infinite recursion in alias expansion: {}", c.join(" -> ")),
			Self::IoError(e) => write!(f, "IO error: {}", e),
			Self::DialoguerError(e) => write!(f, "Dialoguer error: {}", e),
			Self::ParseError(e) => write!(f, "Parse error: {}", e),
			Self::SerializationError(e) => write!(f, "Serialization error: {}", e),
			Self::AmbiguousQuery(q, m) => write!(f, "Multiple results for query \"{}\": {}", q, m.join(", ")),
			Self::NoCommandGiven => write!(f, "No command was supplied"),
			Self::Other(msg) => write!(f, "{msg}"),
		}
	}
}

impl std::error::Error for LauncherError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::IoError(e) => Some(e),
			Self::DialoguerError(e) => Some(e),
			Self::ParseError(e) => Some(e),
			_ => None,
		}
	}
}

impl From<&(dyn std::error::Error + 'static)> for LauncherError {
	fn from(err: &(dyn std::error::Error + 'static)) -> Self {
		// check if it’s an IoError
		if let Some(toml_err) = err.downcast_ref::<toml::de::Error>() {
			LauncherError::ParseError(toml_err.clone())
		} else {
			LauncherError::Other(format!("{}", err))
		}
	}
}

impl From<std::io::Error> for LauncherError {
	fn from(err: std::io::Error) -> Self {
		LauncherError::IoError(err)
	}
}

impl From<dialoguer::Error> for LauncherError {
	fn from(err: dialoguer::Error) -> Self {
		LauncherError::DialoguerError(err)
	}
}

impl From<toml::de::Error> for LauncherError {
	fn from(err: toml::de::Error) -> Self {
		LauncherError::ParseError(err)
	}
}

impl From<toml::ser::Error> for LauncherError {
	fn from(err: toml::ser::Error) -> Self {
		LauncherError::SerializationError(err)
	}
}
