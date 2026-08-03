use serde::Deserialize;
use thisconfig::Config;
use thisconfig::config;
use validator::Validate;

#[config(key = "server")]
#[derive(Clone, Deserialize, Validate)]
struct ServerConfig {
    #[validate(range(min = 1024, max = 65535))]
    port: u16,
    #[validate(length(min = 1))]
    host: String,
    #[validate(range(min = 1, max = 300))]
    timeout: u32,
}

fn main() {
    let config = Config::builder()
        .add_dotenv_file("examples/.env")
        .add_file("examples/config/config.toml")
        .build()
        .expect("Failed to load config file");

    let server_config = config
        .get_validated::<ServerConfig>()
        .expect("Failed to get or validate server config");

    println!("Server Host: {}", server_config.host);
    println!("Server Port: {}", server_config.port);
    println!("Server Timeout: {}", server_config.timeout);
}
