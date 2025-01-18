diesel::table! {
    passwords (id) {
        id -> Integer,
        user_id -> Integer,
        login -> Text,
        site -> Text,
        uppercase -> Bool,
        symbols -> Bool,
        lowercase -> Bool,
        digits -> Bool,
        counter -> Integer,
        version -> Integer,
        length -> Integer,
        created -> Timestamp,
        modified -> Timestamp,
    }
}

diesel::table! {
    tokens (id) {
        id -> Integer,
        user_id -> Integer,
        access_token -> Text,
        refresh_token -> Text,
        created -> Timestamp,
        modified -> Timestamp,
    }
}

diesel::table! {
    users (id) {
        id -> Integer,
        email -> Text,
        password -> Text,
    }
}

diesel::joinable!(passwords -> users (user_id));
diesel::joinable!(tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    passwords,
    tokens,
    users,
);
