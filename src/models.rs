use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug, Insertable)]
#[diesel(table_name = crate::schema::imports)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Import {
    pub id: String,
    pub filename: String,
    pub platform_id: String,
    pub generation_date_unix_timestamp_seconds: i64,
}

#[derive(Queryable, Selectable, Debug, Insertable)]
#[diesel(table_name = crate::schema::accounts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Account {
    pub id: String,
    pub platform_id: String,
    pub import_id: String,
}

#[derive(Queryable, Selectable, Debug, Insertable)]
#[diesel(table_name = crate::schema::transactions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Transaction {
    pub id: Option<i32>,
    pub execution_time_unix_timestamp_seconds: i64,
    pub ticker_symbol: String,
    pub unit_quantity: f64,
    pub cost_per_unit: f64,
    pub currency_symbol: String,
    pub account_id: String,
    pub import_id: String,
}
