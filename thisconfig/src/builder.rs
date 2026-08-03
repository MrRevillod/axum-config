use crate::{Config, ConfigError, ConfigSourceInfo, interpolation::Interpolator};
use std::{fs, path::PathBuf, sync::Arc};
use toml::Table;
use tracing::{error, warn};

#[derive(Debug)]
enum Source {
    File { path: PathBuf, required: bool },
    TomlString { content: String },
}

#[derive(Debug, Default)]
pub struct ConfigBuilder {
    sources: Vec<Source>,
}

impl ConfigBuilder {
    pub fn add_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.sources.push(Source::File {
            path: path.into(),
            required: false,
        });

        self
    }

    pub fn add_required_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.sources.push(Source::File {
            path: path.into(),
            required: true,
        });

        self
    }

    pub fn add_toml_str(mut self, toml: &str) -> Self {
        self.sources.push(Source::TomlString {
            content: toml.to_string(),
        });

        self
    }

    #[cfg(feature = "dotenv")]
    /// Loads environment variables from a specified `.env` file following the
    /// [dotenv](https://crates.io/crates/dotenv) convention.
    pub fn add_dotenv_file<P: Into<PathBuf>>(self, path: P) -> Self {
        dotenv::from_filename(path.into()).ok();
        self
    }

    #[cfg(feature = "dotenv")]
    /// Loads environment variables from a `.env` file following the
    /// [dotenv](https://crates.io/crates/dotenv) convention.
    pub fn add_dotenv(self) -> Self {
        dotenv::dotenv().ok();
        self
    }

    fn load(sources: Vec<Source>) -> Result<Config, ConfigError> {
        let mut merged = Table::new();
        let mut source_infos = Vec::new();

        for source in sources {
            match source {
                Source::File { path, required } => {
                    if path.exists() {
                        source_infos.push(ConfigSourceInfo::File {
                            path: path.clone(),
                            required,
                            found: true,
                        });

                        let content = fs::read_to_string(&path)?;
                        let interpolated = Interpolator::interpolate(&content)
                            .inspect_err(|e| {
                                error!("Interpolation error in file {}: {e}", path.display());
                            })
                            .map_err(ConfigError::interpolation_error)?;

                        let table = toml::from_str::<Table>(&interpolated).inspect_err(|e| {
                            error!("Failed to parse TOML from {}: {}", path.display(), e);
                        })?;

                        Self::merge_tables(&mut merged, table);
                    } else if required {
                        source_infos.push(ConfigSourceInfo::File {
                            path: path.clone(),
                            required,
                            found: false,
                        });

                        error!("Config file not found (required): {}", path.display());

                        return Err(ConfigError::FileNotFound(
                            path.to_str().unwrap_or_default().to_string(),
                        ));
                    } else {
                        source_infos.push(ConfigSourceInfo::File {
                            path: path.clone(),
                            required,
                            found: false,
                        });

                        warn!("Config file not found (optional): {}", path.display());
                    }
                }
                Source::TomlString { content } => {
                    source_infos.push(ConfigSourceInfo::TomlString);

                    let expanded = Interpolator::interpolate(&content)
                        .inspect_err(|e| {
                            error!("Interpolation error in TOML string: {e}");
                        })
                        .map_err(ConfigError::interpolation_error)?;

                    let table: Table = toml::from_str::<Table>(&expanded).inspect_err(|e| {
                        error!("Failed to parse TOML string: {}", e);
                    })?;

                    Self::merge_tables(&mut merged, table);
                }
            }
        }

        Ok(Config {
            inner: Arc::new(merged),
            sources: Arc::from(source_infos),
        })
    }

    fn merge_tables(base: &mut Table, other: Table) {
        for (key, value) in other {
            match base.get_mut(&key) {
                Some(existing)
                    if matches!(existing, toml::Value::Table(_))
                        && matches!(value, toml::Value::Table(_)) =>
                {
                    if let (toml::Value::Table(base_table), toml::Value::Table(other_table)) =
                        (existing, value)
                    {
                        Self::merge_tables(base_table, other_table);
                    }
                }
                _ => {
                    base.insert(key, value);
                }
            }
        }
    }

    /// Builds the configuration from added sources.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if no sources, files missing, or parsing fails.
    pub fn build(self) -> Result<Config, ConfigError> {
        if self.sources.is_empty() {
            return Err(ConfigError::NoSourcesConfigured);
        }

        Self::load(self.sources)
    }
}
