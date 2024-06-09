// @generated automatically by Diesel CLI.

diesel::table! {
    accounts (id) {
        id -> Text,
        platform_id -> Text,
        import_id -> Text,
        label -> Nullable<Text>,
        kind -> Nullable<Text>,
    }
}

diesel::table! {
    imports (id) {
        id -> Text,
        filename -> Text,
        platform_id -> Text,
        generation_date_unix_timestamp_seconds -> BigInt,
    }
}

diesel::table! {
    platforms (id) {
        id -> Text,
        description -> Text,
        url -> Text,
    }
}

diesel::table! {
    transactions (id) {
        id -> Nullable<Integer>,
        execution_time_unix_timestamp_seconds -> BigInt,
        ticker_symbol -> Text,
        unit_quantity -> Double,
        cost_per_unit -> Double,
        currency_symbol -> Text,
        account_id -> Text,
        import_id -> Text,
    }
}

diesel::joinable!(accounts -> imports (import_id));
diesel::joinable!(accounts -> platforms (platform_id));
diesel::joinable!(imports -> platforms (platform_id));
diesel::joinable!(transactions -> accounts (account_id));
diesel::joinable!(transactions -> imports (import_id));

diesel::allow_tables_to_appear_in_same_query!(
    accounts,
    imports,
    platforms,
    transactions,
);
