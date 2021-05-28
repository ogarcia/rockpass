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
    tokens (id) {
        id -> Integer,
        user_id -> Integer,
        token -> Text,
        created -> Timestamp,
        modified -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Integer,
        email -> Text,
        password -> Text,
    }
}

joinable!(passwords -> users (user_id));
joinable!(tokens -> users (user_id));

allow_tables_to_appear_in_same_query!(
    passwords,
    tokens,
    users,
);
