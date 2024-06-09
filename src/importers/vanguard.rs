use std::{collections::HashMap, fs::File, io::BufReader, path::PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use calamine::{open_workbook, Data, DataType, Range, Reader, Xls};
use chrono::{DateTime, TimeZone, Utc};
use diesel::{RunQueryDsl, SelectableHelper, SqliteConnection};
use log::{debug, warn};
use regex::Regex;

use crate::{
    importers::manifest::Platform,
    models::{Account, Import, Transaction},
};

use super::manifest::Manifest;

type Workbook = Xls<BufReader<File>>;

const DATE_CELL_INDEX: (u32, u32) = (1, 0);
const DATE_CELL_NAME: &str = "A2";
const ACCOUNT_NUMBER_CELL_INDEX: (u32, u32) = (3, 0);
const ACCOUNT_NUMBER_CELL_NAME: &str = "A4";

pub(crate) struct Importer {
    manifest: Manifest,

    import_path: PathBuf,
    workbook: Xls<BufReader<File>>,

    sheet_name_by_account_id: HashMap<String, String>,
}

impl Importer {
    pub(crate) fn new(manifest: Manifest, import_path: PathBuf) -> Result<Importer> {
        debug!("opening workbook");
        let workbook = open_workbook::<Workbook, _>(&import_path)?;

        Ok(Importer {
            manifest,
            import_path,
            workbook,
            sheet_name_by_account_id: HashMap::new(),
        })
    }

    pub(crate) fn create_import(&mut self, conn: &mut SqliteConnection) -> Result<Import> {
        use crate::schema::imports;

        let summary_range = self.workbook.worksheet_range("Summary")?;

        let generation_date = report_generation_date(&summary_range)?;
        debug!("report generated at {generation_date:?}");

        let new_import = Import {
            id: self.manifest.sha256sum.clone(),
            filename: self.import_path.to_str().unwrap().to_owned(),
            platform_id: Platform::VanguardUK.id().to_owned(),
            generation_date_unix_timestamp_seconds: generation_date.timestamp(),
        };

        let import = diesel::insert_into(imports::table)
            .values(&new_import)
            .returning(Import::as_returning())
            .get_result(conn)?;
        Ok(import)
    }

    pub(crate) fn create_accounts(
        &mut self,
        conn: &mut SqliteConnection,
        import: &Import,
    ) -> Result<Vec<Account>> {
        use crate::schema::accounts;

        let summary_range = self.workbook.worksheet_range("Summary")?;

        let generation_date = report_generation_date(&summary_range)?;
        debug!("report generated at {generation_date:?}");

        let account_id_re = Regex::new(r".* \((VG.*)\)")?;
        let mut accounts = vec![];
        for (sheet_name, worksheet) in self.workbook.worksheets() {
            if sheet_name == "Summary" {
                continue;
            }
            // The account ID may be in the sheet name. It may also be in the cell A1. FML
            let capture = account_id_re.captures(&sheet_name);
            let account_id = if capture.is_none() {
                if let Some(account_id) =
                    extract_from_string_cell_single(&worksheet, (0, 0), &account_id_re)?
                {
                    account_id
                } else {
                    debug!("no account_id found in {sheet_name:?}");
                    continue;
                }
            } else {
                let (_, [account_id]) = capture.unwrap().extract();
                account_id.to_owned()
            };

            let mut new_account = Account {
                id: account_id.to_string(),
                platform_id: import.platform_id.clone(),
                import_id: import.id.clone(),
                label: None,
                kind: None,
            };
            self.manifest.apply_account_metadata(&mut new_account);
            debug!("associating {} with {}", &new_account.id, sheet_name);
            self.sheet_name_by_account_id
                .insert(new_account.id.clone(), sheet_name);
            accounts.push(new_account);
        }

        diesel::insert_into(accounts::table)
            .values(&accounts)
            .execute(conn)?;

        Ok(accounts)
    }

    pub(crate) fn create_transactions(
        &mut self,
        conn: &mut SqliteConnection,
        import: &Import,
        account: &Account,
    ) -> Result<Vec<Transaction>> {
        /// The per account export sheet is broken up into 2 sections - cash transactions, and then
        /// investment transactions. We are going to scan down the first column until we find a cell
        /// containing "Investment Transactions". The table starts two rows below that.
        use crate::schema::transactions;

        let sheet_name = self
            .sheet_name_by_account_id
            .get(&account.id)
            .ok_or(anyhow!(
                "sheet associated with {} must be present before transactions are created",
                account.id
            ))?;

        let worksheet = self.workbook.worksheet_range(sheet_name)?;

        // Find the investment transactions table
        let mut ix = None;
        for x in 0..1000 {
            match worksheet.get((x, 0)) {
                Some(Data::String(val)) => {
                    if val != "Investment Transactions" {
                        continue;
                    }
                    ix = Some((x, 0));
                    break;
                }
                _ => continue,
            }
        }
        if ix.is_none() {
            warn!("could not find investment transactions for {}", account.id);
            return Ok(vec![]);
        }
        let mut ix = ix.unwrap();
        ix.0 += 3;
        debug!(
            "investment transactions for {} start at {:?}",
            account.id, ix
        );

        let mut transactions = vec![];
        // The row before last is a summary row. Exit when the next row is empty
        while worksheet
            .get((ix.0 + 1, ix.1))
            .is_some_and(|x| !DataType::is_empty(x))
        {
            let new_transaction = read_transaction_row(
                &worksheet.range((ix.0 as u32, ix.1 as u32), (ix.0 as u32, ix.1 as u32 + 5)),
                import,
                account,
            )
            .with_context(|| format!("Failed to read transaction from row {:?}", ix))?;
            let transaction = diesel::insert_into(transactions::table)
                .values(&new_transaction)
                .returning(Transaction::as_returning())
                .get_result(conn)?;
            transactions.push(transaction);
            ix.0 += 1;
        }

        Ok(transactions)
    }
}

fn read_transaction_row(
    row: &Range<Data>,
    import: &Import,
    account: &Account,
) -> Result<crate::models::Transaction> {
    let naive_datetime = row
        .get((0, 0))
        .ok_or(anyhow!("Column A (Date) must be present"))?
        .as_datetime()
        .ok_or(anyhow!("Column A must be date"))?;
    let datetime = Utc
        .from_local_datetime(&naive_datetime)
        .single()
        .ok_or(anyhow!("Timezone fuckery"))?;

    let investment = row
        .get((0, 1))
        .ok_or(anyhow!("Column B (InvestmentName) must be present"))?
        .as_string()
        .ok_or(anyhow!("Column B must be text"))?;

    let quantity = row
        .get((0, 3))
        .ok_or(anyhow!("Column D (Quantity) must be present"))?
        .as_f64()
        .ok_or(anyhow!("Column D must be numeric"))?;
    let price = row
        .get((0, 4))
        .ok_or(anyhow!("Column E (Price) must be present"))?
        .as_f64()
        .ok_or(anyhow!("Column E must be numeric"))?;
    let cost = row
        .get((0, 5))
        .ok_or(anyhow!("Column F (Cost) must be present"))?
        .as_f64()
        .ok_or(anyhow!("Column F must be numeric"))?;
    assert!((cost - price * quantity).abs() < 0.0001);

    Ok(crate::models::Transaction {
        id: None,
        execution_time_unix_timestamp_seconds: datetime.timestamp(),
        ticker_symbol: extract_ticker_symbol(&investment)?.to_owned(),
        unit_quantity: quantity,
        cost_per_unit: price,
        currency_symbol: "GBP".to_owned(),
        account_id: account.id.clone(),
        import_id: import.id.clone(),
    })
}

fn report_generation_date(summary_tab: &Range<Data>) -> Result<DateTime<Utc>> {
    let cell = summary_tab
        .get_value(DATE_CELL_INDEX)
        .ok_or(anyhow!("Could not find any data at {}", DATE_CELL_NAME))?;

    let naive_datetime = cell
        .as_datetime()
        .ok_or(anyhow!("Could not parse date at {}", DATE_CELL_NAME))?;
    Utc.from_local_datetime(&naive_datetime)
        .single()
        .ok_or(anyhow!("Timezone fuckery"))
}

fn extract_ticker_symbol(investment_name: &str) -> Result<&str> {
    let re = Regex::new(r".* \(([A-Z]+)\)$")?;
    let (_, [ticker]) = re
        .captures(investment_name)
        .ok_or(anyhow!(
            "InvestmentName must end in parenthesised ticker symbol: {}",
            investment_name
        ))?
        .extract();

    Ok(ticker)
}

fn extract_from_string_cell<const N: usize>(
    tab: &Range<Data>,
    index: (u32, u32),
    re: &Regex,
) -> Result<Option<[String; N]>> {
    let value = tab
        .get_value(index)
        .ok_or(anyhow!("Could not find cell at {:?}", index))?;
    if !value.is_string() {
        bail!("Cell at {:?} is not a string: {:?}", index, value);
    }
    let cell_contents = value.as_string().unwrap();
    Ok(re
        .captures(&cell_contents)
        .map(|mtch| mtch.extract().1.map(|x| x.to_owned())))
}

fn extract_from_string_cell_single(
    tab: &Range<Data>,
    index: (u32, u32),
    re: &Regex,
) -> Result<Option<String>> {
    if let Some([x]) = extract_from_string_cell(tab, index, re)? {
        Ok(Some(x))
    } else {
        Ok(None)
    }
}
