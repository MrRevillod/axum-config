use axum::{Extension, Router, response::IntoResponse, routing::get};
use serde::Deserialize;
use thisconfig::{Config, ExtractConfig, config};
use tokio::net::TcpListener;

#[config(key = "server")]
#[derive(Clone, Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
}

async fn server_info(ExtractConfig(server): ExtractConfig<ServerConfig>) -> impl IntoResponse {
    format!("Server: {}:{}", server.host, server.port)
}

#[tokio::main]
async fn main() {
    let app_config = Config::builder()
        .add_dotenv_file("examples/.env")
        .add_required_file("examples/config/config.toml")
        .build()
        .expect("Failed to load config file");

    let server_config = app_config.expect::<ServerConfig>();

    let app = Router::new()
        .route("/", get(server_info))
        .layer(Extension(app_config));

    let addr = format!("{}:{}", server_config.host, server_config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    println!("Server running at http://{}", addr);

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
