use transaction::{Transaction, TransactionEngine};

use clap::Parser;
use csv::{ReaderBuilder, WriterBuilder};
use serde::Deserialize;

use std::io::Read;
use std::{fs::File, path::Path};

#[derive(Debug, Parser)]
#[command(about = "Interpreter of CSV transactions", long_about = None)]
struct ProgramArgs {
    // file name for a valid CSV transaction file
    filename: String,
}

// This is a nice hack to make the CSV reader
// and serde deserialize directly to the enum.
// The csv deserializer doesn't directly support
// flat enum: https://github.com/BurntSushi/rust-csv/issues/211
#[derive(Debug, Deserialize)]
struct TransactionWrapper {
    #[serde(flatten)]
    pub transaction: Transaction,
}

fn handle_transactions<R: Read>(reader: R) -> anyhow::Result<TransactionEngine> {
    let mut csv_reader = ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(reader);

    let mut engine = TransactionEngine::default();

    let csv_iterator = csv_reader
        .deserialize::<TransactionWrapper>()
        .map(|res| res.map_err(anyhow::Error::from))
        .map(|res| res.map(|v| v.transaction));

    for result in csv_iterator {
        // Assumption: ignore invalid and malformed transations
        if let Ok(transaction) = result {
            engine.handle(transaction).unwrap_or_else(|err| {
                eprintln!("could not handle transaction {}: {:#?}", transaction, err)
            })
        } else {
            eprintln!("ignoring invalid CSV line: {:?}", result);
            continue;
        }
    }

    Ok(engine)
}

fn main() -> anyhow::Result<()> {
    let args = ProgramArgs::parse();

    let file = File::open(Path::new(&args.filename))?;
    let state = handle_transactions(file)?;

    let mut writer = WriterBuilder::new()
        .flexible(true)
        .from_writer(std::io::stdout());

    for item in state.client_accounts.values() {
        writer.serialize(item)?;
    }

    Ok(())
}

// I guess these should be proper integration tests, but this will do
#[cfg(test)]
mod tests {
    use anyhow::Context;

    use super::*;

    #[test]
    fn parser_happy_path_deposit() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 3.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 2.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_passes_any_spacing_deposit() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 1, 1.0
deposit   , 2, 2, 2.0
    deposit, 1, 3, 2.0"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 3.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 2.0);
        assert!(!client_two.locked);
        Ok(())
    }

    // Ideally, this would work but I dont
    // have time to write my own deserializer
    // with case-insensitiveness
    #[test]
    fn parser_fails_wrong_spelling_deposit() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
Deposit, 1, 1, 1.0
deposiT, 2, 2, 2.0
DEPOSIT, 1, 3, 2.0"#;

        let state = handle_transactions(test_str.as_bytes())?;
        assert!(!state.client_accounts.contains_key(&1u16));
        assert!(!state.client_accounts.contains_key(&2u16));
        Ok(())
    }

    #[test]
    fn parser_happy_path_withdrawal() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_any_spacing_withdrawal() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
    withdrawal, 1, 4, 1.5
withdraw    , 2, 5, 3.0"#;
        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_fails_wrong_spelling_withdrawal() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
Withdrawal, 1, 1, 1.0
WITHDRAWAL, 2, 2, 2.0
withdrawaL, 1, 3, 2.0"#;

        let state = handle_transactions(test_str.as_bytes())?;
        assert!(!state.client_accounts.contains_key(&1u16));
        assert!(!state.client_accounts.contains_key(&2u16));
        Ok(())
    }

    #[test]
    fn parser_happy_path_dispute() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 100, 50
deposit, 2, 42, 50
dispute, 1, 100,
dispute, 2, 42,"#;
        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert_eq!(client_one.held, 50.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert_eq!(client_two.held, 50.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_any_spacing_dispute() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 100, 50
deposit, 2, 42, 50
    dispute, 1, 100,
dispute     , 2, 42,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert_eq!(client_one.held, 50.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert_eq!(client_two.held, 50.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_fails_wrong_spelling_dispute() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 1, 50
deposit, 2, 2, 50
deposit, 3, 3, 50
Dispute, 1, 1,
disputE, 2, 2,
DISPUTE, 3, 3,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 50.0);
        assert_eq!(client_one.held, 0.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 50.0);
        assert_eq!(client_two.held, 0.0);
        assert!(!client_two.locked);

        let client_three = state
            .client_accounts
            .get(&3u16)
            .context("could not get client")?;
        assert_eq!(client_three.available, 50.0);
        assert_eq!(client_three.held, 0.0);
        assert!(!client_three.locked);

        Ok(())
    }

    #[test]
    fn parser_happy_path_resolve() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 100, 50
deposit, 2, 42, 50
dispute, 1, 100,
dispute, 2, 42,
resolve, 1, 100,
resolve, 2, 42,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 50.0);
        assert_eq!(client_one.held, 0.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 50.0);
        assert_eq!(client_two.held, 0.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_any_spacing_resolve() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 1, 100
deposit, 2, 2, 42
dispute, 1, 1,
dispute, 2, 2,
    resolve, 1, 1,
resolve     , 2, 2,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 100.0);
        assert_eq!(client_one.held, 0.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 42.0);
        assert_eq!(client_two.held, 0.0);
        assert!(!client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_fails_wrong_spelling_resolve() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 1, 100
deposit, 2, 2, 42
deposit, 3, 3, 3
dispute, 1, 1,
dispute, 2, 2,
dispute, 3, 3,
Resolve, 1, 1,
resolvE, 2, 2,
RESOLVE, 3, 3,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert_eq!(client_one.held, 100.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert_eq!(client_two.held, 42.0);
        assert!(!client_two.locked);

        let client_three = state
            .client_accounts
            .get(&3u16)
            .context("could not get client")?;
        assert_eq!(client_three.available, 0.0);
        assert_eq!(client_three.held, 3.0);
        assert!(!client_three.locked);

        Ok(())
    }

    #[test]
    fn parser_happy_path_chargeback() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 100, 50
deposit, 2, 42, 50
dispute, 1, 100,
dispute, 2, 42,
chargeback, 1, 100,
chargeback, 2, 42,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert!(client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert!(client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_any_spacing_chargeback() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 100, 50
deposit, 2, 42, 50
dispute, 1, 100,
dispute, 2, 42,
    chargeback, 1, 100,
chargeback     , 2, 42,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 0.0);
        assert_eq!(client_one.held, 0.0);
        assert!(client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 0.0);
        assert_eq!(client_two.held, 0.0);
        assert!(client_two.locked);
        Ok(())
    }

    #[test]
    fn parser_fails_wrong_spelling_chargeback() -> anyhow::Result<()> {
        let test_str = r#"type, client, tx, amount
deposit, 1, 1, 100
deposit, 2, 2, 42
deposit, 3, 3, 3
Chargeback, 1, 1,
chargebacK, 2, 2,
CHARGEBACK, 3, 3,"#;

        let state = handle_transactions(test_str.as_bytes())?;

        let client_one = state
            .client_accounts
            .get(&1u16)
            .context("could not get client")?;
        assert_eq!(client_one.available, 100.0);
        assert!(!client_one.locked);

        let client_two = state
            .client_accounts
            .get(&2u16)
            .context("could not get client")?;
        assert_eq!(client_two.available, 42.0);
        assert!(!client_two.locked);

        let client_three = state
            .client_accounts
            .get(&3u16)
            .context("could not get client")?;
        assert_eq!(client_three.available, 3.0);
        assert!(!client_three.locked);
        Ok(())
    }
    // We can write way more tests here, I just don't have time
}
