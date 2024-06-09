pub mod importers;
pub mod models;
pub mod schema;

use log::debug;
use std::{fs::File, io::Read, path::Path};

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenvy::dotenv;
use std::env;

fn main() -> anyhow::Result<()> {
    dotenv()?;

    let mut clog = colog::default_builder();
    clog.filter(None, log::LevelFilter::Debug);
    clog.init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    debug!("opening db {}", database_url);
    let mut conn = SqliteConnection::establish(&database_url)?;

    crate::importers::vanguard::import_transaction_listing(
        &mut conn,
        Path::new("./imports/Vanguard Transaction Listing.Xls"),
    )?;

    Ok(())
}
