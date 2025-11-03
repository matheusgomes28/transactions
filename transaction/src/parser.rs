use crate::transaction::Transaction;

use serde::Deserialize;
use std::io::Read;

// This is a nice hack to make the CSV reader
// and serde deserialize directly to the enum.
// The csv deserializer doesn't directly support
// flat enum: https://github.com/BurntSushi/rust-csv/issues/211
#[derive(Debug, Deserialize)]
struct TransactionWrapper {
    #[serde(flatten)]
    pub transaction: Transaction,
}

pub fn parse_transactions<T: Read>(reader: T) -> anyhow::Result<Vec<Transaction>> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(reader);

    csv_reader
        .deserialize::<TransactionWrapper>()
        .map(|res| res.map_err(anyhow::Error::from))
        .map(|res| res.map(|v| v.transaction))
        .collect()
}
