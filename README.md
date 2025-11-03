# Transaction

Git repository for a basic transaction verification system, expecting
CSV input for each transaction.

## Assumptions

From the problem description, I had to make a few assumptions on the
input and behaviour of the transaction verifier:

+ We ignore the malformed and invalid transactions.
  + Chargebacks, resolves, disputes cannot be the first time a client is seen, although
  the client is still created.
  + Withdraws cannot happen when an account is locked.
+ CSV input will always be valid!
  + Number of fields in each row is correct, although they can be empty (i.e
  `amount` for chargebacks, resolve, disputes).
  + Spaces are trimmed.
+ The CLI output only cares about the `stdout`. I.e. errors are still
displayed but to `stderr`. If this is a problem, i.e. if using `2>&1`, I
can change that so PLEASE LET ME KNOW!
+ Fraud (locking) **only occurs** when a chargeback happens. There are
other places we can assume fraud, for example, when a customer deposits X,
withdraws X, then disputes the deposit. This was intentionally left
unimplemented as the problem specification did not mention this.
+ Monetary values should be rounded to 4 decimal places, but use `f64`
internally for maximum precision.

## Architecture & Building & Running

This repository contains a Cargo workspace with two coponents:

+ A library `transaction` that exposes the `TransactionEngine` API, as well
as definitions for client account, transactions, etc.
+ A binary `transaction_reader` that uses `transaction` to inject the parsed
`csv`. The biary also contains the main output and logging logic, and many
integration tests.

To build, simply run from the root directory:

```bash
cargo build
```

To run the main application, also run the following from the rood directory:

```bash
cargo run -- ${PATH_TO_CSV} > output.csv
```

which will output the final client account to `output.csv`.

## Dependencies

Here's a list of the main dependencies used and motivation:

+ `csv` used to parse the CSV file. Unfortunately seemed quite limited with `serde`
integration, and some hacks were needed for deserialising directly to enums.
+ `seerde` used in varioud places to support serialisation and deserialisation.
+ `anyhow` used to make cascading error types a little nicer, as well as adding
context to `Option` values.
+ `clap` was used for argument parsing. Although it probably wasn't needed as
the binary only takes a single positional argument. Generated help text is nice,
though.

## Testing & Testing Data

To run tests, simply run the following from the root directory:

```bash
cargo test
```

The e2e testing was largely done in the binary tests, which I've used as a
place to hold integration testing. Therefore, you can find a bunch of the
test data in `transaction_reader/main.rs`

Aside from those, I've used hand-made CSV files for testing, which you can find
in `test_files`.


Thank you for the time, and please give me any useful feedback (even bad ones!).
