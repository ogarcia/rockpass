//
// main.rs
// Copyright (C) 2021 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_migrations;
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;

use rocket::fairing::AdHoc;
use rocket::Rocket;

mod cors;
mod models;
mod routes;
mod schema;

#[database("rockpass")]
pub struct RockpassDatabase(diesel::SqliteConnection);

fn database_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    embed_migrations!();

    let connection = RockpassDatabase::get_one(&rocket).expect("database connection");
    match embedded_migrations::run(&*connection) {
        Ok(()) => Ok(rocket),
        Err(_) => Err(rocket)
    }
}

pub struct RegistrationEnabled(bool);
pub struct TokenLifetime(i64);

fn main() {
    rocket::ignite()
        .attach(cors::Cors)
        .attach(RockpassDatabase::fairing())
        .attach(AdHoc::on_attach("Database Migrations", database_migrations))
        .attach(AdHoc::on_attach("Registration config", |rocket| {
            let registration_enabled = rocket.config()
                .get_bool("registration_enabled")
                .unwrap_or(true);
            Ok(rocket.manage(RegistrationEnabled(registration_enabled)))
        }))
        .attach(AdHoc::on_attach("Token lifetime config", |rocket| {
            let token_lifetime = rocket.config()
                .get_int("token_lifetime")
                .unwrap_or(2592000);
            Ok(rocket.manage(TokenLifetime(token_lifetime)))
        }))
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
        .launch();
}
