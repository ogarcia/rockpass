//
// fairings.rs
// Copyright (C) 2021-2022 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{http::{ContentType, Header}, Data, Request, Response};

pub struct Cors;
pub struct ForceContentType(pub ContentType);

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        // Add CORS headers to allow all origins to all outgoing requests
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "DELETE, GET, OPTIONS, PATCH, POST, PUT"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[rocket::async_trait]
impl Fairing for ForceContentType {
    fn info(&self) -> Info {
        Info {
            name: "Force a ContentType to requests",
            kind: Kind::Request
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _data: &mut Data<'_>) {
        request.replace_header(Header::new("Accept", format!("{}", self.0)));
    }
}
