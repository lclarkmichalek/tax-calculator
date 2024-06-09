use std::{io::Write, path::Path};

use anyhow::{anyhow, Result};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql, FromSqlRow},
    expression::AsExpression,
    prelude::*,
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};
use log::warn;
use serde::Deserialize;

use crate::models::Account;

#[derive(Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
pub(crate) enum Platform {
    #[serde(alias = "vanguard_uk")]
    VanguardUK,
}

impl Platform {
    pub(crate) fn file_extension(&self) -> &'static str {
        match &self {
            Platform::VanguardUK => "Xls",
        }
    }

    pub(crate) fn id(&self) -> &'static str {
        match &self {
            Platform::VanguardUK => "vanguard_uk",
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub sha256sum: String,
    pub platform: Platform,
    pub accounts: Vec<AccountMetadata>,
}

impl Manifest {
    pub(crate) fn validate(&self, import_path: &Path) -> Result<()> {
        let checksum: String = sha256::try_digest(import_path)?;
        if checksum != self.sha256sum {
            return Err(anyhow!(
                "sha256sum for {:?} did not match manifest value: {} != {}",
                import_path,
                checksum,
                self.sha256sum
            ));
        }
        Ok(())
    }

    pub(crate) fn apply_account_metadata(&self, account: &mut Account) {
        for metadata in self.accounts.iter() {
            metadata.apply(account);
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, AsExpression)]
#[sql_type = "diesel::sql_types::Text"]
pub(crate) enum AccountKind {
    #[serde(alias = "isa")]
    ISA,
    #[serde(alias = "gia")]
    GeneralInvestmentAccount,
}

impl AccountKind {
    pub(crate) fn from_id(id: &str) -> Option<AccountKind> {
        match id {
            "isa" => Some(AccountKind::ISA),
            "gia" => Some(AccountKind::GeneralInvestmentAccount),
            _ => {
                warn!("unknown account kind {}", id);
                None
            }
        }
    }

    pub(crate) fn id(&self) -> &'static str {
        match &self {
            AccountKind::ISA => "isa",
            AccountKind::GeneralInvestmentAccount => "gia",
        }
    }
}

impl ToSql<Text, diesel::sqlite::Sqlite> for AccountKind
where
    String: ToSql<Text, diesel::sqlite::Sqlite>,
{
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::sqlite::Sqlite>) -> serialize::Result {
        out.set_value(self.id());
        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for AccountKind
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> deserialize::Result<Self> {
        let val = String::from_sql(bytes)?;
        Ok(AccountKind::from_id(&val)
            .ok_or_else(|| anyhow!("unrecognised AccountKind: {}", val))?)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccountMetadata {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) kind: AccountKind,
}

impl AccountMetadata {
    pub(crate) fn matches(&self, account: &Account) -> bool {
        self.id == account.id
    }

    pub(crate) fn apply(&self, account: &mut Account) {
        if self.matches(account) {
            account.label = Some(self.label.clone());
            account.kind = Some(self.kind);
        }
    }
}
