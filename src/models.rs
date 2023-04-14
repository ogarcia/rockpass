//
// models.rs
// Copyright (C) 2021-2022 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

use rocket::serde::{Serialize, Deserialize, Deserializer};
use chrono::NaiveDateTime;

use crate::schema::passwords;

pub struct AuthorizedUser {
    pub id: i32,
    pub email: String,
    pub password: String,
    pub token_id: i32
}

#[derive(Serialize, Deserialize, Queryable)]
#[serde(crate = "rocket::serde")]
pub struct User {
    pub id: i32,
    pub email: String,
    pub password: String
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewUser {
    pub email: String,
    pub password: String
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UserPassword {
    pub current_password: String
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NewUserPassword {
    pub current_password: String,
    pub new_password: String
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct JWTRefreshToken {
    pub refresh: String
}

#[derive(Serialize, Deserialize, Queryable)]
#[serde(crate = "rocket::serde")]
pub struct DBToken {
    pub id: i32,
    pub user_id: i32,
    pub access_token: String,
    pub refresh_token: String,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime
}

#[derive(Serialize, Deserialize, Queryable)]
#[serde(crate = "rocket::serde")]
pub struct Password {
    pub id: i32,
    pub user_id: i32,
    pub login: String,
    pub site: String,
    pub uppercase: bool,
    pub symbols: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub counter: i32,
    pub version: i32,
    pub length: i32,
    pub created: NaiveDateTime,
    pub modified: NaiveDateTime
}

// Older versions of LessPass send `numbers` instead of `digits`, other versions can even send both
// values, this structure takes care of deserializing this.
#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct DigitsOrNumbers {
    digits: Option<bool>,
    numbers: Option<bool>
}

fn digits_or_numbers<'d, D: Deserializer<'d>>(d: D) -> Result<bool, D::Error> {
    let DigitsOrNumbers { digits, numbers } = DigitsOrNumbers::deserialize(d)?;
    digits.or(numbers).ok_or(rocket::serde::de::Error::missing_field("digits or numbers"))
}

#[derive(Clone, Deserialize, Insertable, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = passwords)]
pub struct NewPassword {
    pub login: String,
    pub site: String,
    pub uppercase: bool,
    pub symbols: bool,
    pub lowercase: bool,
    #[serde(deserialize_with = "digits_or_numbers", flatten)]
    pub numbers: bool,
    pub counter: i32,
    #[serde(default = "default_version")]
    pub version: i32,
    pub length: i32,
}

const fn default_version() -> i32 { 2 }
