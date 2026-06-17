use serde::Deserialize;
use thisconfig::{Config, config};

#[config(key = "app")]
#[derive(Clone, Deserialize, Default)]
struct AppConfig {
    name: String,
    debug: bool,
}

#[config(key = "with-file")]
#[derive(Clone, Deserialize)]
struct WithFileConfig {
    cargo_toml: String,
}

fn main() {
    let config = Config::builder()
        .add_dotenv_file("examples/.env")
        .add_file("nonexistent.toml")
        .add_required_file("examples/config/config.toml")
        .add_toml_str("[extra]\nversion = \"1.0\"")
        .build()
        .expect("Failed to load config file");

    let app_config = config.expect::<AppConfig>();
    let _ = config.get_or_default::<AppConfig>();

    let with_file_config = config.expect::<WithFileConfig>();

    println!("App Name: {}", app_config.name);
    println!("Debug Mode: {}", app_config.debug);

    println!("Cargo.toml Content:\n{}", with_file_config.cargo_toml);
}
