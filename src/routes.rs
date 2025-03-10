//
// routes.rs
// Copyright (C) 2021-2025 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

use bcrypt::{hash, verify};
use chrono::Duration;
use chrono::prelude::*;
use diesel::{self, prelude::*};
use hmac::{Hmac, Mac};
use jwt::{Header, SignWithKey, Token, VerifyWithKey};
use rocket::State;
use rocket::http::Status;
use rocket::request::{Outcome, Request, FromRequest};
use rocket::response::status;
use rocket::serde::json::{Json, Value, json};
use sha2::Sha256;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::models::{AuthorizedUser, NewUser, NewUserPassword, User, UserPassword, JWTRefreshToken, DBToken, NewPassword, Password};
use crate::{RockpassDatabase, RockpassConfig};
use crate::schema::passwords::dsl::*;
use crate::schema::tokens::dsl::*;
use crate::schema::users::dsl::*;
use crate::schema::{passwords, tokens, users};

// Define bcrypt cost for password
const BCRYPT_COST: u32 = 10;

pub struct Authorization(RockpassDatabase, AuthorizedUser);

#[derive(Debug)]
pub enum AuthorizationError {
    Missing,
    Invalid,
    Unauthorized
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Authorization {
    type Error = AuthorizationError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Seek for authorization header
        match request.headers().get_one("authorization") {
            None => Outcome::Error((Status::BadRequest, AuthorizationError::Missing)),
            Some(auth) => {
                // Authorization must start with 'bearer'
                if (auth.len() > 7) && (&auth[..6].to_lowercase()) == "bearer" {
                    // Get database connection
                    let connection = request.guard::<RockpassDatabase>().await.expect("database connection");
                    // Check the autorization token (remove 'bearer' and pass JWT token only)
                    match check_authorization(&connection, &auth[7..]).await {
                        Ok(authorized_user) => Outcome::Success(Authorization(connection, authorized_user)),
                        Err(_) => Outcome::Error((Status::Unauthorized, AuthorizationError::Unauthorized))
                    }
                } else {
                    Outcome::Error((Status::BadRequest, AuthorizationError::Invalid))
                }
            }
        }
    }
}

async fn check_authorization(connection: &RockpassDatabase, authorization_access_token: &str) -> Result<AuthorizedUser, ()> {
    // Get UUID value from authorization token
    let uuid = get_uuid_from_token(&authorization_access_token)?;
    // Seek for UUID field in database
    match connection.run(move |c| {
        tokens::table
            .filter(tokens::access_token.eq(&uuid))
            .load::<DBToken>(c)
    }).await {
        Ok(tokens_vector) => {
            // Validate token and get authorized user
            get_authorized_user(&connection, &tokens_vector, &authorization_access_token).await
        },
        Err(_) => Err(())
    }
}

async fn check_refresh(connection: &RockpassDatabase, payload_refresh_token: &str) -> Result<AuthorizedUser, ()> {
    // Get UUID value from refresh token
    let uuid = get_uuid_from_token(&payload_refresh_token)?;
    // Seek for UUID field in database
    match connection.run(move |c| {
        tokens::table
            .filter(tokens::refresh_token.eq(&uuid))
            .load::<DBToken>(c)
    }).await {
        Ok(tokens_vector) => {
            // Validate token and get authorized user
            get_authorized_user(&connection, &tokens_vector, &payload_refresh_token).await
        },
        Err(_) => Err(())
    }
}

async fn get_authorized_user(connection: &RockpassDatabase, tokens_vector: &Vec<DBToken>, token: &str) -> Result<AuthorizedUser, ()> {
    if tokens_vector.len() == 0 {
        // No UUID token found
        return Err(());
    }
    // Seek for user in database
    let token_user_id = tokens_vector[0].user_id;
    match connection.run(move |c| {
        users::table
            .find(&token_user_id)
            .first::<User>(c)
    }).await {
        Ok(users_vector) => {
            // Check JWT token with shared key and return authorized user
            check_jwt(&users_vector.password, &token.to_string()).
                map(|_| Ok(AuthorizedUser {
                    id: users_vector.id,
                    email: users_vector.email,
                    password: users_vector.password,
                    token_id: tokens_vector[0].id
                }))?
        },
        Err(_) => Err(())
    }
}

fn get_uuid_from_token(token: &str) -> Result<String, ()> {
    // Parse provided token and return UUID
    let jwt_token: Token<Header, BTreeMap<String, String>, _> = Token::parse_unverified(token).map_err(|_| ())?;
    let claims = jwt_token.claims();
    Ok(claims["uuid"].to_string())
}

fn new_jwt(shared_key: &String, uuid: &String, token_lifetime: &i64) -> Result<String, jwt::Error> {
    // Create new HMAC key with shared key
    let key: Hmac<Sha256> = Hmac::new_from_slice(&format!("{}", shared_key).into_bytes()).unwrap();
    // Add UUID into token
    let mut claims = BTreeMap::new();
    claims.insert("uuid", uuid);
    // Add expiration date (in timestamp)
    let expiration_date = (Utc::now() + Duration::seconds(*token_lifetime)).format("%s").to_string();
    claims.insert("exp", &expiration_date);
    // Return new JWT token
    claims.sign_with_key(&key)
}

fn check_jwt(shared_key: &String, jwt_token: &String) -> Result<(), ()> {
    // Create new HMAC key with shared key
    let key: Hmac<Sha256> = Hmac::new_from_slice(&format!("{}", shared_key).into_bytes()).unwrap();
    // Verify token with shared key
    let claims: BTreeMap<String, String> = jwt_token.verify_with_key(&key).map_err(|_| ())?;
    let expiration_date = DateTime::parse_from_str(&claims["exp"], "%s").map_err(|_|())?;
    if expiration_date < Utc::now() {
        Err(()) // Token is valid but expired
    } else {
        Ok(()) // Token is valid
    }
}

async fn create_tokens(connection: &RockpassDatabase, user: &User, access_token_lifetime: &i64, refresh_token_lifetime: &i64) -> Result<(String, String), ()> {
    // Make new UUIDs for access and refresh tokens
    let access_token_uuid = Uuid::new_v4().to_string();
    let refresh_token_uuid = Uuid::new_v4().to_string();
    // Calculate new JWT tokens
    let access_token_jwt = new_jwt(&user.password, &access_token_uuid, &access_token_lifetime).map_err(|_| ())?;
    let refresh_token_jwt = new_jwt(&user.password, &refresh_token_uuid, &refresh_token_lifetime).map_err(|_| ())?;
    // Insert it into database
    let token_user_id = user.id;
    match connection.run(move |c| {
        diesel::insert_into(tokens::table)
            .values((tokens::user_id.eq(&token_user_id), access_token.eq(&access_token_uuid), refresh_token.eq(&refresh_token_uuid)))
            .execute(c)
    }).await {
        Ok(rows) => {
            match rows {
                0 => Err(()),
                // Return the new JWT tokens
                _ => Ok((access_token_jwt, refresh_token_jwt))
            }
        },
        Err(_) => Err(())
    }
}

async fn refresh_tokens(connection: &RockpassDatabase, authorized_user: &AuthorizedUser, access_token_lifetime: &i64, refresh_token_lifetime: &i64) -> Result<(String, String), ()> {
    // Make new UUIDs for access and refresh tokens
    let access_token_uuid = Uuid::new_v4().to_string();
    let refresh_token_uuid = Uuid::new_v4().to_string();
    // Calculate new JWT tokens
    let access_token_jwt = new_jwt(&authorized_user.password, &access_token_uuid, &access_token_lifetime).map_err(|_| ())?;
    let refresh_token_jwt = new_jwt(&authorized_user.password, &refresh_token_uuid, &refresh_token_lifetime).map_err(|_| ())?;
    // Insert it into database
    let token_id = authorized_user.token_id;
    match connection.run(move |c| {
        diesel::update(tokens)
            .filter(tokens::id.eq(&token_id))
            .set((access_token.eq(&access_token_uuid), refresh_token.eq(&refresh_token_uuid), tokens::modified.eq(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())))
            .execute(c)
    }).await {
        Ok(rows) => {
            match rows {
                0 => Err(()),
                // Return the new JWT tokens
                _ => Ok((access_token_jwt, refresh_token_jwt))
            }
        },
        Err(_) => Err(())
    }
}

#[options("/auth/users")]
pub async fn options_auth_users() -> Status {
    Status::NoContent
}

#[post("/auth/users", data = "<user>")]
pub async fn post_auth_users(connection: RockpassDatabase, config: &State<RockpassConfig>, user: Json<NewUser>) -> status::Custom<Json<Value>> {
    if config.registration_enabled {
        // Register new user
        let new_user_email = user.0.email.clone();
        let bcrypted_password = hash(&user.0.password, BCRYPT_COST).unwrap();
        let inserted_rows = match connection.run(move |c| {
            diesel::insert_into(users)
                .values((email.eq(&new_user_email), password.eq(bcrypted_password)))
                .execute(c)
        }).await {
            Ok(rows) => rows,
            Err(_) => 0
        };
        match inserted_rows {
            0 => status::Custom(Status::Conflict, Json(json!({"detail": "User already exists"}))),
            _ => status::Custom(Status::Created, Json(json!({"detail": format!("Created {} user", user.0.email)})))
        }
    } else {
        status::Custom(Status::Forbidden, Json(json!({"detail": "Registration is disabled"})))
    }
}

#[options("/auth/users/me")]
pub async fn options_auth_users_me() -> Status {
    Status::NoContent
}

#[get("/auth/users/me")]
pub async fn get_auth_users_me(authorization: Authorization) -> status::Custom<Json<Value>> {
    status::Custom(Status::Ok, Json(
            json!({
                "id": authorization.1.id,
                "email": authorization.1.email
            })
        ))
}

#[delete("/auth/users/me", data = "<user_password>")]
pub async fn delete_auth_users_me(authorization: Authorization, user_password: Json<UserPassword>) -> status::Custom<Json<Value>> {
    if verify(&user_password.0.current_password, &authorization.1.password).unwrap() {
        let connection = authorization.0;
        let authorized_user_id = authorization.1.id;
        // Delete current user
        match connection.run(move |c| {
            diesel::delete(users)
                .filter(users::id.eq(&authorized_user_id))
                .execute(c)
            }).await {
                Ok(_) => status::Custom(Status::Ok, Json(json!({"detail": "Your user has been deleted"}))),
                Err(_) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem deleting your user"})))
        }
    } else {
        status::Custom(Status::Forbidden, Json(json!({"detail": "Password does not match with the one stored in database"})))
    }
}

#[options("/auth/users/set_password")]
pub async fn options_auth_users_set_password() -> Status {
    Status::NoContent
}

#[post("/auth/users/set_password", data = "<new_user_password>")]
pub async fn post_auth_users_set_password(authorization: Authorization, new_user_password: Json<NewUserPassword>) -> status::Custom<Json<Value>> {
    if verify(&new_user_password.0.current_password, &authorization.1.password).unwrap() {
        let connection = authorization.0;
        let authorized_user_id = authorization.1.id;
        let bcrypted_password = hash(&new_user_password.0.new_password, BCRYPT_COST).unwrap();
        // Change user password
        let updated_rows = match connection.run(move |c| {
            diesel::update(users)
                .filter(users::id.eq(&authorized_user_id))
                .set(password.eq(bcrypted_password))
                .execute(c)
        }).await {
            Ok(rows) => rows,
            Err(_) => 0
        };
        match updated_rows {
            0 => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem updating the password"}))),
            _ => {
                // Delete all user tokens after password change
                let deleted_rows = match connection.run(move |c| {
                    diesel::delete(tokens)
                        .filter(tokens::user_id.eq(&authorized_user_id))
                        .execute(c)
                }).await {
                    Ok(rows) => rows,
                    Err(_) => 0
                };
                status::Custom(Status::Ok, Json(json!({"detail": format!("Password changed for user {} and deleted {} old tokens", authorization.1.email, deleted_rows)})))
            }
        }
    } else {
        status::Custom(Status::Forbidden, Json(json!({"detail": "Old password does not match with the one stored in database"})))
    }
}

#[options("/auth/jwt/create")]
pub async fn options_auth_jwt_create() -> Status {
    Status::NoContent
}

#[post("/auth/jwt/create", data = "<user>")]
pub async fn post_auth_jwt_create(connection: RockpassDatabase, config: &State<RockpassConfig>, user: Json<NewUser>) -> status::Custom<Json<Value>> {
    // Seek for user in database
    let user_email = user.0.email;
    let results: Vec<User> = connection.run(move |c| {
        users::table
            .filter(email.eq(&user_email))
            .limit(1)
            .load::<User>(c)
    }).await.expect("load user");
    // If user found verify password
    if (results.len() == 0) || (! verify(&user.0.password, &results[0].password).unwrap()) {
        return status::Custom(Status::Unauthorized, Json(json!({"detail": "No active account found with the given credentials"})));
    }
    // Generate new token
    match create_tokens(&connection, &results[0], &config.access_token_lifetime, &config.refresh_token_lifetime).await {
        Ok(created_token) => {
            // Delete expired tokens after login
            let min_modification_date = Utc::now() - Duration::seconds(config.refresh_token_lifetime);
            let token_user_id = results[0].id;
            connection.run(move |c| {
                diesel::delete(tokens::table)
                    .filter(tokens::user_id.eq(&token_user_id))
                    .filter(tokens::modified.lt(min_modification_date.format("%Y-%m-%d %H:%M:%S").to_string()))
                    .execute(c)
            }).await.expect("delete expired tokens");
            status::Custom(Status::Created, Json(json!({"access": created_token.0, "refresh": created_token.1})))
        },
        Err(()) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem generating the new token"})))
    }
}

#[options("/auth/jwt/refresh")]
pub async fn options_auth_jwt_refresh() -> Status {
    Status::NoContent
}

#[post("/auth/jwt/refresh", data = "<jwt_refresh_token>")]
pub async fn post_auth_jwt_refresh(connection: RockpassDatabase, config: &State<RockpassConfig>, jwt_refresh_token: Json<JWTRefreshToken>) -> status::Custom<Json<Value>> {
    // Check the refresh token
    match check_refresh(&connection, &jwt_refresh_token.0.refresh).await {
        Ok(authorized_user) => {
            // Generate new token
            match refresh_tokens(&connection, &authorized_user, &config.access_token_lifetime, &config.refresh_token_lifetime).await {
                Ok(refreshed_token) => status::Custom(Status::Created, Json(json!({"access": refreshed_token.0, "refresh": refreshed_token.1}))),
                Err(()) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem generating the new token"})))
            }
        },
        Err(_) => status::Custom(Status::Unauthorized, Json(json!({"detail": "Your refresh token is not valid"})))
    }
}

#[options("/passwords")]
pub async fn options_passwords() -> Status {
    Status::NoContent
}

#[get("/passwords?<search>")]
pub async fn get_passwords(authorization: Authorization, search: Option<String>) -> status::Custom<Json<Value>> {
    let connection = authorization.0;
    // Seek for passwords in database
    let authorized_user_id = authorization.1.id;
    let results: Vec<Password> = match search {
        Some(search) => connection.run(move |c| {
            passwords::table
                .filter(passwords::user_id.eq(&authorized_user_id))
                .filter(passwords::site.like(&format!("%{search}%")))
                .load::<Password>(c)
        }).await.expect("load passwords"),
        None => connection.run(move |c| {
            passwords::table
                .filter(passwords::user_id.eq(&authorized_user_id))
                .load::<Password>(c)
        }).await.expect("load passwords")
    };
    status::Custom(Status::Ok, Json(
            json!({
                "count": results.len(),
                "results": results
            })
        ))
}

#[post("/passwords", data = "<new_password>")]
pub async fn post_passwords(authorization: Authorization, new_password: Json<NewPassword>) -> status::Custom<Json<Value>> {
    let connection = authorization.0;
    // Insert new pasword in database
    let authorized_user_id = authorization.1.id;
    let new_password_to_insert = new_password.0.clone();
    match connection.run(move |c| {
        diesel::insert_into(passwords)
            .values((passwords::user_id.eq(&authorized_user_id), &new_password_to_insert))
            .returning(Password::as_returning())
            .get_result(c)
    }).await {
        Ok(inserted_row) => status::Custom(Status::Created, Json(json!(inserted_row))),
        Err(_) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem creating the new password entry"})))
    }
}

#[options("/passwords/<_password_id>")]
pub async fn options_passwords_id(_password_id: i32) -> Status {
    Status::NoContent
}

#[get("/passwords/<password_id>")]
pub async fn get_passwords_id(authorization: Authorization, password_id: i32) -> status::Custom<Json<Value>> {
    let connection = authorization.0;
    // Seek for passwords in database
    let authorized_user_id = authorization.1.id;
    match connection.run(move |c| {
        passwords::table
            .filter(passwords::id.eq(password_id))
            .filter(passwords::user_id.eq(&authorized_user_id))
            .limit(1)
            .load::<Password>(c)
    }).await {
        Ok(results) => if results.is_empty() {
            status::Custom(Status::NotFound, Json(json!({"detail": format!("Password {password_id} not found in database")})))
        } else {
            status::Custom(Status::Ok, Json(json!(results[0])))
        },
        Err(_) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem getting password entry"})))
    }
}


#[put("/passwords/<updated_password_id>", data = "<updated_password>")]
pub async fn put_passwords_id(authorization: Authorization, updated_password_id: i32, updated_password: Json<NewPassword>) -> status::Custom<Json<Value>> {
    let connection = authorization.0;
    // Update existing password
    let authorized_user_id = authorization.1.id;
    let updated_password_to_insert = updated_password.0.clone();
    let updated_rows = match connection.run(move |c| {
        diesel::update(passwords)
            .filter(passwords::id.eq(updated_password_id))
            .filter(passwords::user_id.eq(&authorized_user_id))
            .set((&updated_password_to_insert, passwords::modified.eq(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())))
        .execute(c)
    }).await {
        Ok(rows) => rows,
        Err(_) => 0
    };
    match updated_rows {
        0 => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem updating the password entry"}))),
        _ => status::Custom(Status::Created, Json(json!({"detail": format!("Updated password entry for site {}", updated_password.0.site)})))
    }
}

#[delete("/passwords/<deleted_password_id>")]
pub async fn delete_passwords_id(authorization: Authorization, deleted_password_id: i32) -> status::Custom<Json<Value>> {
    let connection = authorization.0;
    // Delete existing password
    let authorized_user_id = authorization.1.id;
    let deleted_rows = match connection.run(move |c| {
        diesel::delete(passwords)
            .filter(passwords::id.eq(deleted_password_id))
            .filter(passwords::user_id.eq(&authorized_user_id))
            .execute(c)
    }).await {
        Ok(rows) => rows,
        Err(_) => 0
    };
    match deleted_rows {
        0 => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem deleting the password entry"}))),
        _ => status::Custom(Status::Ok, Json(json!({"detail": format!("Deleted password with id {}", deleted_password_id)})))
    }
}
