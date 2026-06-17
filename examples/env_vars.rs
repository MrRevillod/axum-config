use axum_config::config;
use serde::Deserialize;
use thisconfig::Config;

#[config(key = "env")]
#[derive(Clone, Deserialize, Default)]
struct EnvConfig {
    log_level: String,
    non_defined_env_var: String,
    database_url: String,
}

fn main() {
    let config = Config::builder()
        .add_dotenv_file("examples/.env")
        .add_file("examples/config/config.toml")
        .build()
        .expect("Failed to load config file");

    let env_config = config.expect::<EnvConfig>();

    println!("Log Level: {}", env_config.log_level);
    println!("Non Defined Env Var: {}", env_config.non_defined_env_var);
    println!("Database URL: {}", env_config.database_url);
}
