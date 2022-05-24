//
// main.rs
// Copyright (C) 2021-2022 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

//#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_sync_db_pools;

use serde::Deserialize;
use rocket::fairing::AdHoc;
use rocket::{http::ContentType, Rocket, Build};

mod fairings;
mod models;
mod routes;
mod schema;

#[database("rockpass")]
pub struct RockpassDatabase(diesel::SqliteConnection);

async fn database_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    embed_migrations!();

    let connection = RockpassDatabase::get_one(&rocket).await.expect("database connection");
    connection.run(|c| embedded_migrations::run(c)).await.expect("diesel migrations");

    rocket
}

#[derive(Deserialize)]
pub struct RockpassConfig {
    #[serde(default = "default_registration_enabled")]
    registration_enabled: bool,
    #[serde(default = "default_access_token_lifetime")]
    access_token_lifetime: i64,
    #[serde(default = "default_refresh_token_lifetime")]
    refresh_token_lifetime: i64
}

fn default_registration_enabled() -> bool { true }
fn default_access_token_lifetime() -> i64 { 3600 }
fn default_refresh_token_lifetime() -> i64 { 2592000 }

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(fairings::Cors)
        .attach(fairings::ForceContentType(ContentType::JSON))
        .attach(RockpassDatabase::fairing())
        .attach(AdHoc::config::<RockpassConfig>())
        .attach(AdHoc::on_ignite("Database Migrations", database_migrations))
        .mount("/", routes![
               routes::options_auth_users,
               routes::post_auth_users,
               routes::options_auth_users_set_password,
               routes::post_auth_users_set_password,
               routes::options_auth_jwt_create,
               routes::post_auth_jwt_create,
               routes::options_auth_jwt_refresh,
               routes::post_auth_jwt_refresh,
               routes::options_passwords,
               routes::get_passwords,
               routes::post_passwords,
               routes::options_passwords_id,
               routes::put_passwords_id,
               routes::delete_passwords_id
        ])
}
