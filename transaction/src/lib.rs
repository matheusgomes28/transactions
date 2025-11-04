pub mod client;
pub mod transaction;

pub use client::ClientAccount;
pub use transaction::Transaction;

use std::collections::HashMap;

// Notes on `Ledger` and `AccountStore`:
// Ideally, some chronologically sorted timestamped structure,
// easy to query for an ID. However, the usecase only requires
// a map here and no replay for the accounts and ledger.

// Stores all withdrawalls and deposits
pub type Ledger = HashMap<u64, Transaction>;

// Stores a clients details from the exercise
pub type AccountStore = HashMap<u16, ClientAccount>;

#[derive(Debug, Default)]
pub struct TransactionEngine {
    pub client_accounts: AccountStore,
    pub ledger: Ledger,
}

impl TransactionEngine {
    fn get_or_create_client(&mut self, client_id: u16) -> &mut ClientAccount {
        self.client_accounts
            .entry(client_id)
            .or_insert(ClientAccount::new(client_id))
    }

    pub fn handle(&mut self, transaction: Transaction) -> anyhow::Result<()> {
        // We only need to track the Deposits and Withdrawals in these usecases
        match transaction {
            Transaction::Deposit { transaction_id, .. }
            | Transaction::Withdraw { transaction_id, .. } => {
                if self.ledger.contains_key(&transaction_id) {
                    anyhow::bail!(format!("transaction {} is not unique", transaction_id));
                }

                self.ledger.insert(transaction_id, transaction);
            }

            // We don't need to store the dispute, chargeback, resolves
            // plus they dont have a unique ID for the key and generating
            // one could cause clashes for upcoming transactions. This is
            // a problem I'd solve given more time
            _ => {}
        };

        match transaction {
            Transaction::Deposit {
                client_id, amount, ..
            } => {
                let client_acc = self.get_or_create_client(client_id);

                if amount < 0.0 {
                    anyhow::bail!("cannot deposit negative amount");
                }
                client_acc.available += amount;
                Ok(())
            }

            Transaction::Withdraw {
                client_id, amount, ..
            } => {
                let client_acc = self.get_or_create_client(client_id);

                if amount < 0.0 {
                    anyhow::bail!("cannot withdraw negative amount");
                }

                if client_acc.locked {
                    anyhow::bail!("client account {:?} is locked", client_id);
                }

                if client_acc.available < amount {
                    anyhow::bail!("client account {:?} does not have enough funds", client_id);
                }

                client_acc.available -= amount;
                Ok(())
            }

            Transaction::Dispute {
                transaction_id,
                client_id: dispute_client_id,
                ..
            } => {
                // Is a dispute ever valid for a withdrawal???
                if let Some(Transaction::Deposit {
                    client_id: transaction_client_id,
                    amount,
                    disputed,
                    ..
                }) = self.ledger.get_mut(&transaction_id)
                {
                    if dispute_client_id != *transaction_client_id {
                        anyhow::bail!(
                            "client {} does not have transaction id {}",
                            dispute_client_id,
                            *transaction_client_id
                        );
                    }

                    if let Some(ClientAccount {
                        held, available, ..
                    }) = self.client_accounts.get_mut(transaction_client_id)
                    {
                        if available < amount {
                            anyhow::bail!(
                                "client {} does not enough funds to dispute",
                                transaction_client_id
                            );
                        }

                        *held += *amount;
                        *available -= *amount;
                        *disputed = true;
                    } else {
                        // cannot be the fisrt time were seeing this client
                        self.get_or_create_client(dispute_client_id);
                        anyhow::bail!(
                            "could not find client {} for the dispute, creating it",
                            dispute_client_id
                        );
                    }

                    return Ok(());
                }

                anyhow::bail!("disputed transaction {} does not exist", transaction_id);
            }

            Transaction::Resolve {
                transaction_id,
                client_id: dispute_client_id,
                ..
            }
            | Transaction::Chargeback {
                transaction_id,
                client_id: dispute_client_id,
                ..
            } => {
                if let Some(Transaction::Deposit {
                    client_id: transaction_client_id,
                    amount,
                    disputed,
                    ..
                }) = self.ledger.get_mut(&transaction_id)
                {
                    if *transaction_client_id != dispute_client_id {
                        anyhow::bail!(
                            "client {} does not have transaction id {}",
                            *transaction_client_id,
                            dispute_client_id
                        );
                    }

                    if let Some(ClientAccount {
                        held,
                        available,
                        locked,
                        ..
                    }) = self.client_accounts.get_mut(transaction_client_id)
                    {
                        if !*disputed {
                            anyhow::bail!("transaction {} has not been disputed", transaction_id);
                        }

                        // These only differ in these operations
                        if matches!(transaction, Transaction::Resolve { .. }) {
                            if held < amount {
                                anyhow::bail!(
                                    "client {} does not enough held funds to resolve",
                                    transaction_client_id
                                );
                            }

                            *available += *amount;
                            *held -= *amount;
                            *disputed = false;
                        }

                        if matches!(transaction, Transaction::Chargeback { .. }) {
                            *held -= *amount;
                            *locked = true;
                            *disputed = false;
                        }
                    } else {
                        // cannot be the fisrt time were seeing this client
                        self.get_or_create_client(dispute_client_id);
                        anyhow::bail!(
                            "could not find client {} for the dispute, creating it",
                            dispute_client_id
                        );
                    }

                    return Ok(());
                }

                anyhow::bail!("transaction");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn creates_client_after_deposit() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 15.7;

        let mut engine = TransactionEngine::default();

        let transaction = Transaction::Deposit {
            transaction_id: 100,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(transaction)?;

        let client = engine
            .client_accounts
            .get(&client_id)
            .context("client does not exist")?;

        assert_eq!(client.available, deposit_amount);
        Ok(())
    }

    #[test]
    fn creates_client_after_withdraw() -> anyhow::Result<()> {
        let client_id = 10;
        let amount = 15.7;

        let mut engine = TransactionEngine::default();

        let transaction = Transaction::Withdraw {
            transaction_id: 100,
            client_id,
            amount,
        };

        // Expected to error
        engine.handle(transaction).unwrap_or_default();

        let client = engine
            .client_accounts
            .get(&client_id)
            .context("client does not exist")?;

        assert_eq!(client.available, 0.0);
        Ok(())
    }

    #[test]
    fn withdraws_valid_amount() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let withdraw_amount = 50.25;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id: 100,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        let withdraw = Transaction::Withdraw {
            transaction_id: 50,
            client_id,
            amount: withdraw_amount,
        };
        engine.handle(withdraw)?;

        let client = engine
            .client_accounts
            .get(&client_id)
            .context("client does not exist")?;

        assert_eq!(client.available, deposit_amount - withdraw_amount);
        assert_eq!(client.held, 0.0);
        assert!(!client.locked);
        Ok(())
    }

    #[test]
    fn withdraws_invalid_amount() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let withdraw_amount = 150.25;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id: 100,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        // Expected to fail
        let withdraw = Transaction::Withdraw {
            transaction_id: 50,
            client_id,
            amount: withdraw_amount,
        };
        engine.handle(withdraw).unwrap_or_default();

        let client = engine
            .client_accounts
            .get(&client_id)
            .context("client does not exist")?;

        assert_eq!(client.available, deposit_amount);
        assert_eq!(client.held, 0.0);
        assert!(!client.locked);
        Ok(())
    }

    #[test]
    fn withdraws_ignored_when_locked() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let withdraw_amount = 50.25;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id: 100,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        // Mock the account being locked
        {
            let client = engine
                .client_accounts
                .get_mut(&client_id)
                .context("client does not exist")?;
            client.locked = true;
        }

        // Try withdraw valid amount - Expected fail
        let withdraw = Transaction::Withdraw {
            transaction_id: 50,
            client_id,
            amount: withdraw_amount,
        };
        engine.handle(withdraw).unwrap_or_default();

        let client = engine
            .client_accounts
            .get(&client_id)
            .context("client does not exist")?;

        assert_eq!(client.available, deposit_amount);
        assert_eq!(client.held, 0.0);
        assert!(client.locked);
        Ok(())
    }

    #[test]
    fn disputes_valid_transaction() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        // Now dispute it
        let dispute = Transaction::Dispute {
            transaction_id,
            client_id,
        };
        engine.handle(dispute)?;

        Ok(())
    }

    #[test]
    fn disputes_invalid_transaction() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        // Now dispute it
        let dispute = Transaction::Dispute {
            transaction_id: transaction_id + 1,
            client_id,
        };

        // Expected to fail
        assert!(engine.handle(dispute).is_err());
        Ok(())
    }

    #[test]
    fn disputes_withdrawn_amount() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let withdraw_amount = 50.0;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        let withdraw = Transaction::Withdraw {
            transaction_id: 101,
            client_id,
            amount: withdraw_amount,
        };
        engine.handle(withdraw)?;

        // Now dispute it
        let dispute = Transaction::Dispute {
            transaction_id,
            client_id,
        };
        assert!(engine.handle(dispute).is_err());

        // TODO : This is clearly a fraud, but I'm unsure
        // if this is part of the assignment. This should
        // result in the account being locked but curent
        // implementation doesnt assume bad intent.

        Ok(())
    }

    #[test]
    fn resolves_valid_transaction() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        let dispute = Transaction::Dispute {
            transaction_id,
            client_id,
        };
        engine.handle(dispute)?;

        let resolve = Transaction::Resolve {
            transaction_id,
            client_id,
        };
        engine.handle(resolve)?;

        Ok(())
    }

    #[test]
    fn resolves_undisputed_transaction() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        let resolve = Transaction::Resolve {
            transaction_id,
            client_id,
        };
        assert!(engine.handle(resolve).is_err());

        Ok(())
    }

    #[test]
    fn chargeback_valid_transaction() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        let dispute = Transaction::Dispute {
            transaction_id,
            client_id,
        };
        engine.handle(dispute)?;

        // Make sure we're not locked yet
        {
            let client = engine.client_accounts.get(&client_id).unwrap();
            assert!(!client.locked);
        }

        let chargeback = Transaction::Chargeback {
            transaction_id,
            client_id,
        };
        engine.handle(chargeback)?;

        // Make sure we're locked
        {
            let client = engine.client_accounts.get(&client_id).unwrap();
            assert!(client.locked);
        }

        Ok(())
    }

    #[test]
    fn chargeback_undisputed_transaction() -> anyhow::Result<()> {
        let client_id = 10;
        let deposit_amount = 100.5;
        let transaction_id = 100;

        let mut engine = TransactionEngine::default();

        // Deposit then withdraw
        let deposit = Transaction::Deposit {
            transaction_id,
            client_id,
            amount: deposit_amount,
            disputed: false,
        };
        engine.handle(deposit)?;

        {
            let client = engine.client_accounts.get(&client_id).unwrap();
            assert!(!client.locked);
        }

        let chargeback = Transaction::Chargeback {
            transaction_id,
            client_id,
        };
        assert!(engine.handle(chargeback).is_err());

        {
            let client = engine.client_accounts.get(&client_id).unwrap();
            assert!(!client.locked);
        }

        Ok(())
    }
}
