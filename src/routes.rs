//
// routes.rs
// Copyright (C) 2021 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::Duration;
use chrono::prelude::*;
use diesel::{self, prelude::*};
use hmac::{Hmac, NewMac};
use jwt::{Header, SignWithKey, Token, VerifyWithKey};
use rocket::{Outcome, State};
use rocket::http::Status;
use rocket::request::{self, Request, FromRequest};
use rocket::response::status;
use rocket::response::Response;
use rocket_contrib::json::{Json, JsonValue};
use sha2::Sha256;
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::models::{NewUser, User, NewPassword, Password};
use crate::{RockpassDatabase, RegistrationEnabled};
use crate::schema::users::dsl::*;
use crate::schema::{passwords, passwords::dsl::*};

pub struct Authorization(RockpassDatabase, User);

#[derive(Debug)]
pub enum AuthorizationError {
    Missing,
    Invalid
}

impl<'a, 'r> FromRequest<'a, 'r> for Authorization {
    type Error = AuthorizationError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        // Seek for authorization header
        match request.headers().get_one("authorization") {
            None => Outcome::Failure((Status::BadRequest, AuthorizationError::Missing)),
            Some(auth) => {
                // Authorization must start with 'bearer'
                if (auth.len() > 7) && (&auth[..6].to_lowercase()) == "bearer" {
                    // Create database connection
                    let connection = request.guard::<RockpassDatabase>().expect("database connection");
                    // Check the autorization token (remove 'bearer' and pass JWT token only)
                    match check_authorization(&connection, &auth[7..]) {
                        Ok(authorized_user) => Outcome::Success(Authorization(connection, authorized_user)),
                        Err(_) => Outcome::Failure((Status::BadRequest, AuthorizationError::Invalid))
                    }
                } else {
                    Outcome::Failure((Status::BadRequest, AuthorizationError::Invalid))
                }
            }
        }
    }
}

fn check_authorization(connection: &diesel::SqliteConnection, authorization_token: &str) -> Result<User, ()> {
    // Parse provided token
    let jwt_token: Token<Header, BTreeMap<String, String>, _> = Token::parse_unverified(authorization_token).map_err(|_| ())?;
    let claims = jwt_token.claims();
    // Seek for UUID field in database
    match users.filter(token.eq(&claims["uuid"])).load::<User>(connection) {
        Ok(users_vector) => {
            if users_vector.len() == 0 {
                // No user with UUID token found
                return Err(());
            }
            // Check JWT token with shared key and return user
            check_jwt(&users_vector[0].password, &authorization_token.to_string()).
                map(|_| Ok(User {
                    id: users_vector[0].id,
                    email: format!("{}", users_vector[0].email),
                    password: format!("{}", users_vector[0].password),
                    token: format!("{}", users_vector[0].token)
                }))?
        },
        Err(_) => Err(())
    }
}

fn new_jwt(shared_key: &String, uuid: &String) -> String {
    // Create new HMAC key with shared key
    let key: Hmac<Sha256> = Hmac::new_varkey(&format!("{}", shared_key).into_bytes()).unwrap();
    // Add UUID into token
    let mut claims = BTreeMap::new();
    claims.insert("uuid", uuid);
    // Add expiration date (in timestamp)
    let expiration_date = (Utc::now() + Duration::days(30)).format("%s").to_string();
    claims.insert("exp", &expiration_date);
    // Return new JWT token
    claims.sign_with_key(&key).unwrap()
}

fn check_jwt(shared_key: &String, jwt_token: &String) -> Result<(), ()> {
    // Create new HMAC key with shared key
    let key: Hmac<Sha256> = Hmac::new_varkey(&format!("{}", shared_key).into_bytes()).unwrap();
    // Verify token with shared key
    let claims: BTreeMap<String, String> = jwt_token.verify_with_key(&key).map_err(|_| ())?;
    let expiration_date = Utc.datetime_from_str(&claims["exp"], "%s").map_err(|_|())?;
    if expiration_date < Utc::now() {
        Err(()) // Token is valid but expired
    } else {
        Ok(()) // Token is valid
    }
}

fn refresh_token(connection: &diesel::SqliteConnection, user: &User) -> Result<String, ()> {
    // Make new UUID for user token
    let uuid = Uuid::new_v4().to_string();
    // Calculate new JWT token
    let jwt = new_jwt(&user.password, &uuid);
    // Insert it into database
    match diesel::update(users.find(&user.id))
        .set(token.eq(&uuid))
        .execute(connection) {
            Ok(rows) => {
                match rows {
                    0 => Err(()),
                    // Return the new JWT token
                    _ => Ok(jwt)
                }
            },
            Err(_) => Err(())
        }
}

#[options("/auth/users")]
pub fn options_auth_users<'a>() -> Response<'a> {
    Response::build().status(Status::NoContent).finalize()
}

#[post("/auth/users", data = "<user>")]
pub fn post_auth_users(connection: RockpassDatabase, user: Json<NewUser>, registration_enabled: State<RegistrationEnabled>) -> status::Custom<Json<JsonValue>> {
    if registration_enabled.0 {
        // Register new user
        let uuid = Uuid::new_v4().to_string();
        let bcrypted_password = hash(&user.0.password, DEFAULT_COST).unwrap();
        let inserted_rows = match diesel::insert_into(users)
            .values((email.eq(&user.0.email), password.eq(bcrypted_password), token.eq(uuid)))
            .execute(&connection.0) {
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

#[options("/auth/jwt/create")]
pub fn options_auth_jwt_create<'a>() -> Response<'a> {
    Response::build().status(Status::NoContent).finalize()
}

#[post("/auth/jwt/create", data = "<user>")]
pub fn post_auth_jwt_create(connection: RockpassDatabase, user: Json<NewUser>) -> status::Custom<Json<JsonValue>> {
    // Seek for user in database
    let results: Vec<User> = users.filter(email.eq(&user.0.email))
        .limit(1)
        .load::<User>(&connection.0)
        .expect("load user");
    // If user found verify password
    if (results.len() == 0) || (! verify(&user.0.password, &results[0].password).unwrap()) {
        return status::Custom(Status::Unauthorized, Json(json!({"detail": "No active account found with the given credentials"})));
    }
    // Generate new token
    match refresh_token(&connection.0, &results[0]) {
        Ok(refreshed_token) => status::Custom(Status::Created, Json(json!({"access": refreshed_token}))),
        Err(()) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem generating the new token"})))
    }
}

#[options("/auth/jwt/refresh")]
pub fn options_auth_jwt_refresh<'a>() -> Response<'a> {
    Response::build().status(Status::NoContent).finalize()
}


#[post("/auth/jwt/refresh")]
pub fn post_auth_jwt_refresh(authorized_user: Authorization) -> status::Custom<Json<JsonValue>> {
    // Generate new token
    match refresh_token(&authorized_user.0, &authorized_user.1) {
        Ok(refreshed_token) => status::Custom(Status::Created, Json(json!({"access": refreshed_token}))),
        Err(()) => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem generating the new token"})))
    }
}

#[options("/passwords")]
pub fn options_passwords<'a>() -> Response<'a> {
    Response::build().status(Status::NoContent).finalize()
}

#[get("/passwords")]
pub fn get_passwords(authorization: Authorization) -> status::Custom<Json<JsonValue>> {
    let connection = authorization.0;
    // Seek for passwords in database
    let results: Vec<Password> = passwords.filter(user_id.eq(&authorization.1.id))
          .load::<Password>(&connection.0)
          .expect("load passwords");
    status::Custom(Status::Ok, Json(
            json!({
                "count": results.len(),
                "results": results
            })
        ))
}

#[post("/passwords", data = "<new_password>")]
pub fn post_passwords(authorization: Authorization, new_password: Json<NewPassword>) -> status::Custom<Json<JsonValue>> {
    let connection = authorization.0;
    // Insert new pasword in database
    let inserted_rows = match diesel::insert_into(passwords)
        .values((user_id.eq(&authorization.1.id), &new_password.0))
        .execute(&connection.0) {
            Ok(rows) => rows,
            Err(_) => 0
        };
    match inserted_rows {
        0 => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem creating the new password entry"}))),
        _ => status::Custom(Status::Created, Json(json!({"detail": format!("Created new password entry for site {}", new_password.0.site)})))
    }
}

#[options("/passwords/<_password_id>")]
pub fn options_passwords_id<'a>(_password_id: i32) -> Response<'a> {
    Response::build().status(Status::NoContent).finalize()
}

#[put("/passwords/<updated_password_id>", data = "<updated_password>")]
pub fn put_passwords_id(authorization: Authorization, updated_password_id: i32, updated_password: Json<NewPassword>) -> status::Custom<Json<JsonValue>> {
    let connection = authorization.0;
    // Update existing password
    let updated_rows = match diesel::update(passwords)
        .filter(passwords::id.eq(updated_password_id))
        .filter(user_id.eq(&authorization.1.id))
        .set((&updated_password.0, modified.eq(Utc::now().format("%Y-%m-%d %H:%M:%S").to_string())))
        .execute(&connection.0) {
            Ok(rows) => rows,
            Err(_) => 0
        };
    match updated_rows {
        0 => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem updating the password entry"}))),
        _ => status::Custom(Status::Created, Json(json!({"detail": format!("Updated password entry for site {}", updated_password.0.site)})))
    }
}

#[delete("/passwords/<deleted_password_id>")]
pub fn delete_passwords_id(authorization: Authorization, deleted_password_id: i32) -> status::Custom<Json<JsonValue>> {
    let connection = authorization.0;
    // Delete existing password
    let deleted_rows = match diesel::delete(passwords)
        .filter(passwords::id.eq(deleted_password_id))
        .filter(user_id.eq(&authorization.1.id))
        .execute(&connection.0) {
            Ok(rows) => rows,
            Err(_) => 0
        };
    match deleted_rows {
        0 => status::Custom(Status::InternalServerError, Json(json!({"detail": "There was a problem deleting the password entry"}))),
        _ => status::Custom(Status::Ok, Json(json!({"detail": format!("Deleted password with id {}", deleted_password_id)})))
    }
}
