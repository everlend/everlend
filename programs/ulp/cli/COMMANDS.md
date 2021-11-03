# Commands
```
cargo run create-market --keypair locnet_keypair.json
cargo run create-pool --market 4P2WtU2RayKhRc1pfjJP5M9JmVVWZQi91za2ugJvHumG --token CV3A2AbeKc4CoRRcyWwe96LkPktpaPnUAgnzqJVy6wKf
cargo run create-pool-borrow-authority --borrower FjvDD58C8Su9Uq92dztpUpAkoY9dzAf3HiwUxbpMkcru --pool CEDGxQ1Hga4LsCHAHme1C4RJM9fimTfwjUfokPM5YU8Q
```

```
cargo run update-pool-borrow-authority --borrower FjvDD58C8Su9Uq92dztpUpAkoY9dzAf3HiwUxbpMkcru --pool CEDGxQ1Hga4LsCHAHme1C4RJM9fimTfwjUfokPM5YU8Q --share 0.7
```

```
cargo run delete-pool-borrow-authority --borrower FjvDD58C8Su9Uq92dztpUpAkoY9dzAf3HiwUxbpMkcru --pool CEDGxQ1Hga4LsCHAHme1C4RJM9fimTfwjUfokPM5YU8Q
```

```
cargo run create-market --keypair locnet_keypair.json && \
spl-token create-token t1_keypair.json && \
spl-token create-token t2_keypair.json && \
cargo run create-pool --market 4P2WtU2RayKhRc1pfjJP5M9JmVVWZQi91za2ugJvHumG --token CV3A2AbeKc4CoRRcyWwe96LkPktpaPnUAgnzqJVy6wKf && \
cargo run create-pool --market 4P2WtU2RayKhRc1pfjJP5M9JmVVWZQi91za2ugJvHumG --token HhYrKswJaQt9XNyRzbv1aujgC6EPrQ2BKMhGuPPqFmAg && \
cargo run create-pool-borrow-authority --borrower FjvDD58C8Su9Uq92dztpUpAkoY9dzAf3HiwUxbpMkcru --pool CEDGxQ1Hga4LsCHAHme1C4RJM9fimTfwjUfokPM5YU8Q
```