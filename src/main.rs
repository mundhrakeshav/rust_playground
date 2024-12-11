use axum;
use tokio::net::TcpListener;
mod routes;
mod services;
mod auth;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Unable to connect to the server");
    let app = routes::app().await;
    
    println!("Listening on {}", listener.local_addr().unwrap() );
    axum::serve(listener, app)
        .await
        .expect("Error serving application");

}