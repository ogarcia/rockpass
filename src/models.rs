//
// models.rs
// Copyright (C) 2021 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

use serde::{Serialize, Deserialize};
use chrono::NaiveDateTime;

use crate::schema::passwords;

#[derive(Serialize, Deserialize, Queryable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub password: String,
    pub token: String
}

#[derive(Serialize, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub password: String
}

#[derive(Serialize, Deserialize, Queryable)]
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

#[derive(Serialize, Deserialize, Insertable, AsChangeset)]
#[table_name="passwords"]
pub struct NewPassword {
    pub login: String,
    pub site: String,
    pub uppercase: bool,
    pub symbols: bool,
    pub lowercase: bool,
    pub numbers: bool,
    pub counter: i32,
    pub version: i32,
    pub length: i32,
}
