//
// cors.rs
// Copyright (C) 2021 Óscar García Amor <ogarcia@connectical.com>
// Distributed under terms of the GNU GPLv3 license.
//

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{http::Header, Request, Response};

pub struct Cors;

impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    fn on_response(&self, _: &Request, response: &mut Response) {
        // Add CORS headers to allow all origins to all outgoing requests
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "DELETE, GET, OPTIONS, PATCH, POST, PUT"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}
