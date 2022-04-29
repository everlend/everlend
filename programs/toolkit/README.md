# Toolkit

A package for deploying and testing contracts in live networks

### Generating accounts
    $ RUST_LOG=solana=debug cargo run create -A accounts.devnet.yaml --mints SOL USDC USDT

Same, but do not rewrite accounts.devnet.yaml

    $ RUST_LOG=solana=debug cargo run create --mints SOL

### Run tests
    $ RUST_LOG=solana=debug cargo run test full
