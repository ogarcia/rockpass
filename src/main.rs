//
// main.rs
// Copyright (C) 2021-2025 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

#[macro_use] extern crate diesel;
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
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    RockpassDatabase::get_one(&rocket).await
        .expect("database connection")
        .run(|c| { c.run_pending_migrations(MIGRATIONS).expect("diesel migrations"); })
        .await;

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
        .merge(Serialized::default("databases.rockpass.url", ":memory:"))
        .merge(Toml::file("/etc/rockpass.toml").nested())
        .merge(Toml::file("rockpass.toml").nested())
        .merge(Env::prefixed("ROCKPASS_").global())
        .select(Profile::from_env_or("ROCKPASS_PROFILE", "release"));

    rocket::custom(figment)
        .attach(fairings::Cors)
        .attach(fairings::ForceContentType(ContentType::JSON))
        .attach(RockpassDatabase::fairing())
        .attach(AdHoc::config::<RockpassConfig>())
        .attach(AdHoc::on_ignite("Database Migrations", database_migrations))
        .mount("/", routes![
               routes::options_auth_users,
               routes::post_auth_users,
               routes::options_auth_users_me,
               routes::get_auth_users_me,
               routes::delete_auth_users_me,
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

#[cfg(test)]
mod tests {
    use super::rocket;
    use rocket::http::{ContentType, Header, Status};
    use rocket::local::asynchronous::Client;
    use rocket::serde::Deserialize;

    use crate::models::Password;

    #[derive(Deserialize)]
    #[serde(crate = "rocket::serde")]
    struct Token {
        access: String,
        refresh: String
    }

    #[derive(Deserialize)]
    #[serde(crate = "rocket::serde")]
    struct Passwords {
        count: u8,
        results: Vec<Password>
    }

    async fn create_user(client: &Client) {
        // Create a sample user for tests that need it as a requirement
        client.post("/auth/users")
            .header(ContentType::JSON)
            .body(r#"{"email":"test@rockpass.sample","password":"test"}"#)
            .dispatch().await;
    }

    async fn create_token(client: &Client) -> Token {
        // Create a sample user and authenticate it by returning its access and refresh token
        create_user(client).await;
        let request = client.post("/auth/jwt/create")
            .header(ContentType::JSON)
            .body(r#"{"email":"test@rockpass.sample","password":"test"}"#);
        let response = request.dispatch().await;
        response.into_json::<Token>().await.unwrap()
    }

    async fn create_passwords(client: &Client, token: &Token) {
        client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"alice@rockpass.sample","site":"rockpass.sample","uppercase":true,"symbols":true,"lowercase":true,"digits":true,"counter":1,"version":2,"length":16}"#)
            .dispatch().await;
        client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"bob@rockpass.sample","site":"subsite.rockpass.sample","uppercase":true,"symbols":false,"lowercase":true,"digits":true,"counter":2,"version":2,"length":16}"#)
            .dispatch().await;
        client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"charlie@rockpass.sample","site":"other.rockpass.sample","uppercase":true,"symbols":false,"lowercase":true,"digits":false,"counter":1,"version":2,"length":8}"#)
            .dispatch().await;
    }

    #[rocket::async_test]
    async fn test_post_auth_users() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a sample user
        let request = client.post("/auth/users")
            .header(ContentType::JSON)
            .body(r#"{"email":"test@rockpass.sample","password":"test"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Created test@rockpass.sample user"}"#);
        // Create a sample user again, it must fail because the user already exists
        let request = client.post("/auth/users")
            .header(ContentType::JSON)
            .body(r#"{"email":"test@rockpass.sample","password":"test"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Conflict);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"User already exists"}"#);
    }

    #[rocket::async_test]
    async fn test_get_auth_users_me() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to get user data fails because no access token specified
        let request = client.get("/auth/users/me")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to get user data fails because, although a valid formatted token is
        // specified, the token is not correct.
        let request = client.get("/auth/users/me")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "bearer false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // Get user data
        let request = client.get("/auth/users/me")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await.unwrap(), r#"{"email":"test@rockpass.sample","id":1}"#);
    }

    #[rocket::async_test]
    async fn test_delete_auth_users_me() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to delete user fails because no access token specified
        let request = client.delete("/auth/users/me")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to delete user fails because, although a valid formatted token is specified,
        // the token is not correct.
        let request = client.delete("/auth/users/me")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "bearer false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // The attempt to delete user fails because, although it has been validated correctly, the
        // password does not match.
        let request = client.delete("/auth/users/me")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"current_password":"bad"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Forbidden);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Password does not match with the one stored in database"}"#);
        // Get user data
        let request = client.delete("/auth/users/me")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"current_password":"test"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Your user has been deleted"}"#);
    }

    #[rocket::async_test]
    async fn test_post_auth_jwt_create() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Attempts to create a token but fails because the user does not yet exist
        let request = client.post("/auth/jwt/create")
            .header(ContentType::JSON)
            .body(r#"{"email":"test@rockpass.sample","password":"test"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"No active account found with the given credentials"}"#);
        // The example user is created
        create_user(&client).await;
        // Create a token
        let request = client.post("/auth/jwt/create")
            .header(ContentType::JSON)
            .body(r#"{"email":"test@rockpass.sample","password":"test"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
        let _token = response.into_json::<Token>().await.unwrap();
    }

    #[rocket::async_test]
    async fn test_post_auth_jwt_refresh() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempts to refresh the token but fails because the refresh token is invalid
        let request = client.post("/auth/jwt/refresh")
            .header(ContentType::JSON)
            .body(r#"{"refresh":"false"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Your refresh token is not valid"}"#);
        // Token is refreshed
        let request = client.post("/auth/jwt/refresh")
            .header(ContentType::JSON)
            .body(format!(r#"{{"refresh":"{}"}}"#, token.refresh));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
        let _token = response.into_json::<Token>().await.unwrap();
    }

    #[rocket::async_test]
    async fn test_post_auth_users_set_password() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to change password fails because no access token specified
        let request = client.post("/auth/users/set_password")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // Attempt to change password fails because no valid access token specified
        let request = client.post("/auth/users/set_password")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to change the password fails because, although a valid formatted token is
        // specified, the token is not correct.
        let request = client.post("/auth/users/set_password")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.refresh)));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // The attempt to change the password fails because, although a valid token is specified,
        // there is no message body.
        let request = client.post("/auth/users/set_password")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to change the password fails because, although it has been validated
        // correctly, the old password does not match.
        let request = client.post("/auth/users/set_password")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"current_password":"bad","new_password":"bad"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Forbidden);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Old password does not match with the one stored in database"}"#);
        // Password is changed
        let request = client.post("/auth/users/set_password")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"current_password":"test","new_password":"new"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Password changed for user test@rockpass.sample and deleted 1 old tokens"}"#);
    }

    #[rocket::async_test]
    async fn test_post_passwords() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to add password fails because no access token specified
        let request = client.post("/passwords")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to add the password fails because, although a valid formatted token is
        // specified, the token is not correct.
        let request = client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "bearer false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // The attempt to add the password fails because, although a valid token is specified,
        // the message body is invalid.
        let request = client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"schema":"bad"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::UnprocessableEntity);
        // Password is added with old model
        let request = client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"alice@rockpass.sample","site":"rockpass.sample","uppercase":true,"symbols":true,"lowercase":true,"numbers":true,"counter":1,"version":2,"length":16}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Created new password entry for site rockpass.sample"}"#);
        // Password is added with both models
        let request = client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"bob@rockpass.sample","site":"bob.rockpass.sample","uppercase":true,"symbols":true,"lowercase":true,"digits":true,"numbers":true,"counter":1,"length":16}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
        // Password is added with new model
        let request = client.post("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"charlie@rockpass.sample","site":"charlie.rockpass.sample","uppercase":true,"symbols":true,"lowercase":true,"digits":true,"counter":1,"length":16}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
    }

    #[rocket::async_test]
    async fn test_put_passwords_id() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to update password fails because no access token specified
        let request = client.put("/passwords/1")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to update the password fails because, although a valid formatted token is
        // specified, the token is not correct.
        let request = client.put("/passwords/1")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "bearer false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // Create some passwords
        create_passwords(&client, &token).await;
        // The attempt to update the password fails because, although a valid token is specified,
        // the message body is invalid.
        let request = client.put("/passwords/1")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"schema":"bad"}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::UnprocessableEntity);
        // The attempt to update the password fails because, although a valid token is specified
        // and the scheme is valid, the identifier does not exist.
        let request = client.put("/passwords/100")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"alice@rockpass.sample","site":"rockpass.sample","uppercase":true,"symbols":true,"lowercase":true,"digits":true,"counter":1,"version":2,"length":16}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::InternalServerError);
        // Check that the password to be changed has the following default values
        let request = client.get("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        let passwords = response.into_json::<Passwords>().await.unwrap();
        assert_eq!(passwords.results[0].login, "alice@rockpass.sample");
        assert_eq!(passwords.results[0].site, "rockpass.sample");
        assert_eq!(passwords.results[0].uppercase, true);
        assert_eq!(passwords.results[0].counter, 1);
        // Password is updated
        let request = client.put("/passwords/1")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)))
            .body(r#"{"login":"alice@newmail.rockpass.sample","site":"rockpass.sample","uppercase":false,"symbols":true,"lowercase":true,"digits":true,"counter":2,"version":2,"length":16}"#);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Created);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Updated password entry for site rockpass.sample"}"#);
        // Check that the password has been updated correctly
        let request = client.get("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        let passwords = response.into_json::<Passwords>().await.unwrap();
        assert_eq!(passwords.results[0].login, "alice@newmail.rockpass.sample");
        assert_eq!(passwords.results[0].site, "rockpass.sample");
        assert_eq!(passwords.results[0].uppercase, false);
        assert_eq!(passwords.results[0].counter, 2);
    }

    #[rocket::async_test]
    async fn test_delete_passwords_id() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to delete password fails because no access token specified
        let request = client.delete("/passwords/1")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to delete the password fails because, although a valid formatted token is
        // specified, the token is not correct.
        let request = client.delete("/passwords/1")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "bearer false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // Create some passwords
        create_passwords(&client, &token).await;
        // The attempt to delete the password fails because, although a valid token is specified
        // and the scheme is valid, the identifier does not exist.
        let request = client.delete("/passwords/100")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::InternalServerError);
        // Get the password counter, there must be 3
        let request = client.get("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        let passwords = response.into_json::<Passwords>().await.unwrap();
        assert_eq!(passwords.count, 3);
        // Password is deleted
        let request = client.delete("/passwords/1")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.into_string().await.unwrap(), r#"{"detail":"Deleted password with id 1"}"#);
        // Obtain the password counter, there should be 2 left
        let request = client.get("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        let passwords = response.into_json::<Passwords>().await.unwrap();
        assert_eq!(passwords.count, 2);
    }

    #[rocket::async_test]
    async fn test_get_passwords() {
        let client = Client::tracked(rocket()).await.unwrap();
        // Create a user and token
        let token = create_token(&client).await;
        // Attempt to get password fails because no access token specified
        let request = client.get("/passwords")
            .header(ContentType::JSON);
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::BadRequest);
        // The attempt to get the password fails because, although a valid formatted token is
        // specified, the token is not correct.
        let request = client.get("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", "bearer false"));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Unauthorized);
        // Create some passwords
        create_passwords(&client, &token).await;
        // Get passwords
        let request = client.get("/passwords")
            .header(ContentType::JSON)
            .header(Header::new("authorization", format!("bearer {}", token.access)));
        let response = request.dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let passwords = response.into_json::<Passwords>().await.unwrap();
        assert_eq!(passwords.count, 3);
        assert_eq!(passwords.results[0].uppercase, true);
        assert_eq!(passwords.results[1].symbols, false);
        assert_eq!(passwords.results[1].counter, 2);
        assert_eq!(passwords.results[2].length, 8);
        assert_eq!(passwords.results[0].login, "alice@rockpass.sample");
        assert_eq!(passwords.results[1].login, "bob@rockpass.sample");
        assert_eq!(passwords.results[2].login, "charlie@rockpass.sample");
    }
}
