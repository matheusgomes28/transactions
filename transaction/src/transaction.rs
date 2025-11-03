use serde::Deserialize;
use std::cmp::PartialEq;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Transaction {
    Deposit {
        #[serde(rename = "tx")]
        transaction_id: u64,
        #[serde(rename = "client")]
        client_id: u16,
        amount: f64,

        // For internal use to track whether
        // this transaction has been disputed
        #[serde(skip_deserializing, default)]
        disputed: bool,
    },
    #[serde(alias = "withdrawal")]
    Withdraw {
        #[serde(rename = "tx")]
        transaction_id: u64,
        #[serde(rename = "client")]
        client_id: u16,
        amount: f64,
    },
    Dispute {
        #[serde(rename = "tx")]
        transaction_id: u64,
        #[serde(rename = "client")]
        client_id: u16,
    },
    Resolve {
        #[serde(rename = "tx")]
        transaction_id: u64,
        #[serde(rename = "client")]
        client_id: u16,
    },
    Chargeback {
        #[serde(rename = "tx")]
        transaction_id: u64,
        #[serde(rename = "client")]
        client_id: u16,
    },
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // I only want to display the variant name in the error for now
        match self {
            Transaction::Deposit { .. } => write!(f, "Deposit"),
            Transaction::Withdraw { .. } => write!(f, "Withdraw"),
            Transaction::Dispute { .. } => write!(f, "Dispute"),
            Transaction::Resolve { .. } => write!(f, "Resolve"),
            Transaction::Chargeback { .. } => write!(f, "Chargeback"),
        }
    }
}
