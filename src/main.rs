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

use rocket::fairing::AdHoc;
use rocket::figment::{Figment, Profile, providers::{Env, Format, Serialized, Toml}};
use rocket::{http::ContentType, Rocket, Build};
use rocket::serde::{Deserialize, Serialize};

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

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct RockpassConfig {
    registration_enabled: bool,
    access_token_lifetime: i64,
    refresh_token_lifetime: i64
}

impl Default for RockpassConfig {
    fn default() -> RockpassConfig {
        RockpassConfig {
            registration_enabled: true,
            access_token_lifetime: 3600,
            refresh_token_lifetime: 2592000
        }
    }
}

#[launch]
fn rocket() -> _ {
    let figment = Figment::from(rocket::Config::default())
        .merge(Serialized::defaults(RockpassConfig::default()))
        .merge(Toml::file("/etc/rockpass.toml").nested())
        .merge(Toml::file("rockpass.toml").nested())
        .merge(Env::prefixed("ROCKPASS_").global())
        .select(Profile::from_env_or("ROCKPASS_PROFILE", "release"));
    let database_url = match figment.extract_inner::<String>("databases.rockpass.url") {
        Ok(database_url) => database_url,
        Err(_) => ":memory:".to_string()
    };
    let figment = figment.merge(("databases.rockpass.url", database_url));

    rocket::custom(figment)
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
