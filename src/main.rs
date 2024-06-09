pub mod importers;
pub mod models;
pub mod schema;

use anyhow::Context;
use log::{debug, info};
use std::{path::Path};

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

    for (manifest, import_path) in crate::importers::find_imports(Path::new("./imports"))? {
        info!(
            "processing import {:?} ({:?})",
            import_path, manifest.platform
        );
        info!("validating manifest");
        manifest.validate(import_path.as_path())?;

        let mut importer = crate::importers::vanguard::Importer::new(manifest, import_path)?;
        info!("creating import record");
        let import = importer.create_import(&mut conn)?;
        info!("importing accounts");
        let accounts = importer.create_accounts(&mut conn, &import)?;
        debug!(
            "imported {} account records: {:?}",
            accounts.len(),
            accounts
        );
        info!("importing transactions");
        for account in accounts {
            let transactions = importer
                .create_transactions(&mut conn, &import, &account)
                .with_context(|| {
                    format!("failed to import transactions for account {}", account.id)
                })?;
            debug!(
                "imported {} transactions for {}",
                transactions.len(),
                account.id
            );
        }
    }

    Ok(())
}
