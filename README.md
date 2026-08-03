# ThisConfig - Config Loader

Configuration management for Rust applications. Load configuration from TOML files with support for environment variable interpolation and multi-file merging.

## Features

- Load configuration from TOML files
- Multi file support with merging and overriding
- Environment variable interpolation (`${VAR}` and `${VAR:default}`)
- File loading directly on config files (`key = "file:path"`)
- Axum integration via feature flag

## Installation

```bash
cargo add thisconfig
```

## Usage

### 1. Define configuration structure

```rust
use serde::Deserialize;
use thisconfig::config;

#[config(key = "database")]
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
}
```

### 2. Create configuration file

`config/config.toml`:

```toml
[database]
host = "${DB_HOST:localhost}"
port = 5432
user = "${DB_USER:admin}"
```

### 3. Use in your application

````rust
use thisconfig::Config;

#[tokio::main]
async fn main() {
    let config = Config::builder()
        .add_file("config/config.toml")
        .build()
        .expect("Failed to load config");

    let db_config = config.expect::<DatabaseConfig>();

    println!("DB: {}:{}", db_config.host, db_config.port);
}
```

## Configuration Loading

Use `Config::builder()` to specify configuration files. Files can be optional (loaded if present) or required (must exist). You can also add TOML strings directly.

```rust
let config = Config::builder()
    .add_file("config/config.toml")  // optional
    .add_required_file("secrets.toml")  // required
    .add_toml_str("[extra]\nkey = 'value'")  // in-memory TOML
    .build()?;
````

Files are merged in order, with later files overriding earlier ones.

## Environment variables

Supports interpolation in TOML with default values:

```toml
[server]
host = "${HOST:0.0.0.0}"      # Uses HOST or defaults to "0.0.0.0"
port = "${PORT}"              # Requires PORT to be defined
database_url = "${DATABASE_URL}/my_database"  # Requires DATABASE_URL and appends "/my_database"
```

## Config Methods

| Method                | Description                                                |
| --------------------- | ---------------------------------------------------------- |
| `get<T>()`            | Returns the configuration section as `Option<T>`           |
| `get_or_default<T>()` | Returns the config section or default if missing           |
| `expect<T>()`         | Returns the config section or panics if missing            |
| `get_validated<T>()`  | Returns the config section or validation errors if invalid |

> **Note**: Enable the `validation` feature in your `Cargo.toml` for `get_validated<T>()` support. This requires your config structs to implement `Validate` from the `validator` crate.

## Examples

See the [examples](./examples) directory for complete working examples of using `thisconfig`.
