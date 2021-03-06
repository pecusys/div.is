#[macro_use]
extern crate serde;
extern crate argonautica;
mod db;
mod api;
mod models;

use warp::{Filter, self};
use warp::http::Method;
use api::routes;

#[tokio::main]
async fn main() -> sqlx::Result<()> {
    
    let (host, port) = hosts();
    let db = db::setup().await?;

    let cors = warp::cors()
        .allow_credentials(true)
        .allow_any_origin()
        .allow_headers(vec![
            "Authorization", "Access-Control-Allow-Origin", "content-type", "credentials",
        ])
        .allow_methods(&[
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
        ]);

    let app_data = api::AppData {
        jwt_secret: api::auth::get_jwt_secret().await.unwrap(),
        secret_key: api::auth::get_secret_key().await.unwrap(),
        db,
    };

    let routes = warp::path("api")
        .and(routes::routes(app_data))
        .with(cors);

    warp::serve(routes).run(([0, 0, 0, 0], 3001)).await;
    Ok(())
}

// TODO handle this more gracefully

pub fn hosts() -> (String, String) {
    let host = dotenv::var("DEV_HOST").expect("DEV_HOST not set");
    let port = dotenv::var("DEV_PORT").unwrap_or("3001".to_string());
    (host, port) 
}

//TODO: merge routes and handlers, maybe? $08/02/20$
//TODO  proper error handling and enumeration $08/02/20$
//TODO: check to see if path -> using -> vars -> map works for ordering
