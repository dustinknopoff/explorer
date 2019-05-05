table! {
    notes (id) {
        id -> BigInt,
        title -> Nullable<Text>,
    }
}

table! {
    tags (tag) {
        noteId -> BigInt,
        tag -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    notes,
    tags,
);
