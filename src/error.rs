#![allow(dead_code)]

// --- imports ---
use std::fmt;

// --- definitions ---
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

// --- implementations ---
impl fmt::Display for LauncherError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::InvalidAlias(alias, target) => write!(f, "invalid alias {alias} -> {target}"),
			Self::InvalidConfig(reason) => {
				write!(f, "invalid config")?;
				if let Some(r) = reason {
					write!(f, " -- {r}")?;
				}
				Ok(())
			}
			Self::InvalidApp(path, reason) => {
				write!(f, "invalid app definition in {path}")?;
				if let Some(r) = reason {
					write!(f, " -- {r}")?;
				}
				Ok(())
			}
			Self::ConfigNotFound(path) => write!(f, "config file not found in {path}"),
			Self::AppNotFound(name) => write!(f, "app definition not found for {name}"),
			Self::AliasNotFound(name) => write!(f, "alias \"{name}\" was not found"),
			Self::CircularAlias(c) => write!(f, "infinite recursion in alias expansion: {}", c.join(" -> ")),
			Self::IoError(e) => write!(f, "io error: {}", e),
			Self::DialoguerError(e) => write!(f, "dialoguer error: {}", e),
			Self::ParseError(e) => write!(f, "parse error: {}", e),
			Self::SerializationError(e) => write!(f, "serialization error: {}", e),
			Self::AmbiguousQuery(q, m) => write!(f, "multiple results for query \"{}\": {}", q, m.join(", ")),
			Self::NoCommandGiven => write!(f, "no command was supplied"),
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
