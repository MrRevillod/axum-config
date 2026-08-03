use serde::Deserialize;
use thisconfig::Config;
use thisconfig::{ByteConfig, TimeConfig, config};

#[config(key = "app")]
#[derive(Clone, Deserialize)]
struct AppConfig {
    name: String,
    max_size: ByteConfig,
    timeout: TimeConfig,
}

fn main() {
    let toml_config = r#"
[app]
name = "UnitExample"
max_size = "5MB"
timeout = "2m 30s"
"#;

    let config = Config::builder()
        .add_toml_str(toml_config)
        .build()
        .expect("Failed to load config");

    let app_config = config.expect::<AppConfig>();

    println!("App: {}", app_config.name);
    println!(
        "Max Size: {} bytes (raw: {})",
        app_config.max_size.parsed, app_config.max_size.raw
    );
    println!(
        "Timeout: {:.2}s (raw: {})",
        app_config.timeout.parsed.as_secs_f64(),
        app_config.timeout.raw
    );
}
