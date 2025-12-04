# Free Tunnel Solana

```bash
# build the program
cargo build-sbf

# clean the old local chain and run a new local chain
solana-test-validator --reset

# .. or continue the old local chain
solana-test-validator

# deploy the program
solana program deploy target/deploy/free_tunnel_solana.so

# initialize the program
node scripts/1-initialize.js

# create token
spl-token create-token

# Creating token GGTNAp3YA3FxsvBxKPuZ4iqrGhB66H96k8Rx4xxm3E7z under program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
# Address:  GGTNAp3YA3FxsvBxKPuZ4iqrGhB66H96k8Rx4xxm3E7z
# Decimals:  9


```