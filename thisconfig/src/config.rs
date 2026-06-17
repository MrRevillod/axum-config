#[cfg(feature = "validation")]
use crate::ConfigError;
use crate::{ConfigBuilder, ConfigItem};
use serde::de::{DeserializeOwned, IntoDeserializer};
use std::{path::Path, path::PathBuf, sync::Arc};
use toml::{Table, Value};

#[cfg(feature = "validation")]
use validator::Validate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSourceInfo {
    File {
        path: PathBuf,
        required: bool,
        found: bool,
    },
    TomlString,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub(crate) inner: Arc<Table>,
    pub(crate) sources: Arc<[ConfigSourceInfo]>,
}

impl Config {
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Returns source provenance in build order.
    pub fn sources(&self) -> &[ConfigSourceInfo] {
        &self.sources
    }

    /// Iterates only existing file sources used during build.
    pub fn file_sources(&self) -> impl Iterator<Item = &Path> {
        self.sources.iter().filter_map(|source| match source {
            ConfigSourceInfo::File { path, found, .. } if *found => Some(path.as_path()),
            _ => None,
        })
    }

    /// Returns the first existing file source used during build.
    pub fn primary_file_source(&self) -> Option<&Path> {
        self.file_sources().next()
    }

    /// Retrieves a configuration section.
    ///
    /// # Returns
    ///
    /// `Some(T)` if found, `None` otherwise.
    pub fn get<T: DeserializeOwned + ConfigItem>(&self) -> Option<T> {
        let key = T::key();

        let item = self.inner.get(key).cloned()?;
        let value = Value::into_deserializer(item);

        match T::deserialize(value) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::error!(config_key = %key, error = %e, "Failed to deserialize config section");
                None
            }
        }
    }

    #[cfg(feature = "validation")]
    /// Retrieves and validates a configuration section.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` for missing keys or validation errors.
    pub fn get_validated<T>(&self) -> Result<T, ConfigError>
    where
        T: DeserializeOwned + ConfigItem + Validate,
    {
        let key = T::key();

        let item = self
            .inner
            .get(key)
            .cloned()
            .ok_or_else(|| ConfigError::KeyNotFound {
                key: key.to_string(),
            })?;

        let value = Value::into_deserializer(item);

        let deserialized: T = T::deserialize(value)?;

        deserialized
            .validate()
            .map_err(|e| ConfigError::ValidationError {
                message: format!("Validation failed for '{key}': {e}"),
            })?;

        Ok(deserialized)
    }

    /// Retrieves a required configuration section, panicking if not found or invalid.
    ///
    /// # Panics
    /// Panics if the configuration section is missing or cannot be deserialized. Recommended for
    /// critical configuration items that must be present for the application to function. For optional
    /// items, use `get` or `get_or_default` instead.
    pub fn expect<T: DeserializeOwned + ConfigItem>(&self) -> T {
        self.get::<T>()
            .unwrap_or_else(|| panic!("Failed to load configuration for key '{}'", T::key()))
    }

    /// Retrieves a configuration section, returning default if not found or invalid.
    ///
    /// # Returns
    ///
    /// The section or its default.
    pub fn get_or_default<T: DeserializeOwned + ConfigItem + Default>(&self) -> T {
        self.get::<T>().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConfigBuilder, ConfigError};
    use serde::Deserialize;
    use std::fs;

    #[derive(Debug, Clone, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        port: u16,
    }

    impl ConfigItem for TestConfig {
        fn key() -> &'static str {
            "test"
        }
    }

    #[test]
    fn test_from_path_valid_toml() {
        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path();
        fs::write(path, "[test]\nname = \"myapp\"\nport = 8080").expect("failed to write");

        let config = Config::builder()
            .add_required_file(path)
            .build()
            .expect("failed to load config");
        let test_config = config
            .get::<TestConfig>()
            .expect("failed to get test config");

        assert_eq!(test_config.name, "myapp");
        assert_eq!(test_config.port, 8080);
    }

    #[test]
    fn test_from_path_nonexistent_file() {
        let result = Config::builder()
            .add_required_file("/nonexistent/path/config.toml")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_get_missing_key() {
        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path();
        fs::write(path, "[other]\nvalue = 1").expect("failed to write");

        let config = Config::builder()
            .add_required_file(path)
            .build()
            .expect("failed to load config");
        let result = config.get::<TestConfig>();

        assert!(result.is_none());
    }

    #[test]
    fn test_macro_config() {
        use crate::ConfigItem;

        #[derive(Debug, Clone, Deserialize)]
        struct MacroConfig {
            value: String,
        }

        impl ConfigItem for MacroConfig {
            fn key() -> &'static str {
                "macro_test"
            }
        }

        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path();
        fs::write(path, "[macro_test]\nvalue = \"works\"").expect("failed to write");

        let config = Config::builder()
            .add_required_file(path)
            .build()
            .expect("failed to load config");
        let macro_config = config
            .get::<MacroConfig>()
            .expect("failed to get macro config");

        assert_eq!(macro_config.value, "works");
    }

    #[cfg(feature = "validation")]
    #[test]
    fn test_get_validated_success() {
        use crate::ConfigItem;
        use validator::Validate;

        #[derive(Debug, Clone, Deserialize, Validate)]
        struct ValidConfigTest {
            #[validate(length(min = 1))]
            name: String,
            #[validate(range(min = 1, max = 100))]
            value: u32,
        }

        impl ConfigItem for ValidConfigTest {
            fn key() -> &'static str {
                "valid_test"
            }
        }

        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path();
        fs::write(path, "[valid_test]\nname = \"test\"\nvalue = 50").expect("failed to write");

        let config = Config::builder()
            .add_required_file(path)
            .build()
            .expect("failed to load config");
        let valid_config = config
            .get_validated::<ValidConfigTest>()
            .expect("failed to get validated config");

        assert_eq!(valid_config.name, "test");
        assert_eq!(valid_config.value, 50);
    }

    #[cfg(feature = "validation")]
    #[test]
    fn test_get_validated_failure() {
        use crate::ConfigItem;
        use validator::Validate;

        #[derive(Debug, Clone, Deserialize, Validate)]
        struct InvalidConfigTest {
            #[validate(length(min = 3))]
            name: String,
        }

        impl ConfigItem for InvalidConfigTest {
            fn key() -> &'static str {
                "invalid_test"
            }
        }

        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path();
        fs::write(path, "[invalid_test]\nname = \"a\"").expect("failed to write");

        let config = Config::builder()
            .add_required_file(path)
            .build()
            .expect("failed to load config");
        let result = config.get_validated::<InvalidConfigTest>();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Validation failed"));
        }
    }

    #[test]
    fn test_builder_add_file() {
        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path();
        fs::write(path, "[test]\nname = \"builder\"\nport = 9000").expect("failed to write");

        let config = Config::builder()
            .add_file(path)
            .build()
            .expect("failed to build config");

        let test_config = config
            .get::<TestConfig>()
            .expect("failed to get test config");

        assert_eq!(test_config.name, "builder");
        assert_eq!(test_config.port, 9000);
    }

    #[test]
    fn test_builder_add_required_file_missing() {
        let result = Config::builder()
            .add_required_file("/nonexistent/path/config.toml")
            .build();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, ConfigError::FileNotFound(_)));
        }
    }

    #[test]
    fn test_builder_merge_order() {
        let temp_file1 = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path1 = temp_file1.path();
        fs::write(path1, "[test]\nname = \"first\"\nport = 8080").expect("failed to write");

        let temp_file2 = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path2 = temp_file2.path();
        fs::write(path2, "[test]\nname = \"second\"").expect("failed to write");

        let config = ConfigBuilder::default()
            .add_file(path1)
            .add_file(path2)
            .build()
            .expect("failed to build config");

        let test_config = config
            .get::<TestConfig>()
            .expect("failed to get test config");

        assert_eq!(test_config.name, "second");
        assert_eq!(test_config.port, 8080);
    }

    #[test]
    fn test_builder_add_toml_str() {
        let toml_str = r#"
[test]
name = "toml_str"
port = 9999
"#;

        let config = Config::builder()
            .add_toml_str(toml_str)
            .build()
            .expect("failed to build config");

        let test_config = config
            .get::<TestConfig>()
            .expect("failed to get test config");

        assert_eq!(test_config.name, "toml_str");
        assert_eq!(test_config.port, 9999);
    }

    #[test]
    fn test_builder_deep_merge() {
        let temp_file1 = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path1 = temp_file1.path();
        fs::write(
            path1,
            r#"
[test]
name = "app"
port = 8080

[test.nested]
key1 = "value1"
"#,
        )
        .expect("failed to write");

        let temp_file2 = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path2 = temp_file2.path();
        fs::write(
            path2,
            r#"
[test]
port = 9000

[test.nested]
key2 = "value2"
"#,
        )
        .expect("failed to write");

        let config = Config::builder()
            .add_file(path1)
            .add_file(path2)
            .build()
            .expect("failed to build config");

        assert!(config.inner.get("test").is_some());
    }

    #[test]
    fn test_sources_preserve_order_and_found_status() {
        let missing = PathBuf::from("/tmp/definitely-missing-thisconfig.toml");
        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path().to_path_buf();
        fs::write(&path, "[test]\nname = \"ordered\"\nport = 7000").expect("failed to write");

        let config = Config::builder()
            .add_file(&missing)
            .add_toml_str("[test]\nname = \"inline\"")
            .add_file(&path)
            .build()
            .expect("failed to build config");

        assert_eq!(config.sources().len(), 3);
        assert!(matches!(
            &config.sources()[0],
            ConfigSourceInfo::File {
                path,
                required: false,
                found: false
            } if path == &missing
        ));
        assert!(matches!(&config.sources()[1], ConfigSourceInfo::TomlString));
        assert!(matches!(
            &config.sources()[2],
            ConfigSourceInfo::File {
                path: p,
                required: false,
                found: true
            } if p == &path
        ));
    }

    #[test]
    fn test_file_sources_only_existing_files() {
        let missing = PathBuf::from("/tmp/definitely-missing-thisconfig-2.toml");
        let temp_file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path = temp_file.path().to_path_buf();
        fs::write(&path, "[test]\nname = \"files\"\nport = 7001").expect("failed to write");

        let config = Config::builder()
            .add_file(&missing)
            .add_toml_str("[test]\nname = \"inline\"")
            .add_file(&path)
            .build()
            .expect("failed to build config");

        let files: Vec<&Path> = config.file_sources().collect();
        assert_eq!(files, vec![path.as_path()]);
    }

    #[test]
    fn test_primary_file_source_first_existing_file() {
        let missing = PathBuf::from("/tmp/definitely-missing-thisconfig-3.toml");
        let temp_file1 = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path1 = temp_file1.path().to_path_buf();
        fs::write(&path1, "[test]\nname = \"first\"\nport = 7100").expect("failed to write");

        let temp_file2 = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let path2 = temp_file2.path().to_path_buf();
        fs::write(&path2, "[test]\nname = \"second\"\nport = 7200").expect("failed to write");

        let config = Config::builder()
            .add_file(&missing)
            .add_file(&path1)
            .add_file(&path2)
            .build()
            .expect("failed to build config");

        assert_eq!(config.primary_file_source(), Some(path1.as_path()));
    }
}
