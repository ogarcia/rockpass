table! {
    passwords (id) {
        id -> Integer,
        user_id -> Integer,
        login -> Text,
        site -> Text,
        uppercase -> Bool,
        symbols -> Bool,
        lowercase -> Bool,
        numbers -> Bool,
        counter -> Integer,
        version -> Integer,
        length -> Integer,
        created -> Timestamp,
        modified -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Integer,
        email -> Text,
        password -> Text,
        token -> Text,
    }
}

joinable!(passwords -> users (user_id));

allow_tables_to_appear_in_same_query!(
    passwords,
    users,
);
