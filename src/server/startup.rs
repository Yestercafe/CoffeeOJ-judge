use std::net::TcpListener;

use actix_web::{dev::Server, web, App, HttpServer};
use tracing_actix_web::TracingLogger;

pub struct WebApp {
    server: Server,
    port: u16,
}

pub static HOST: &str = "0.0.0.0";
pub static PORT: &str = "4514";

impl WebApp {
    pub async fn new() -> Result<Self, std::io::Error> {
        let address = format!("{}:{}", HOST, PORT);
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();
        let server = create_server(listener)?;
        Ok(WebApp { server, port })
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }
}

pub fn create_server(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| {
        App::new()
            .wrap(TracingLogger::default())
            .route("/", web::get().to(crate::server::routes::index))
            .route(
                "/api/v1/submit",
                web::post().to(crate::server::routes::api::submit),
            )
    })
    .listen(listener)?
    .run();
    Ok(server)
}
