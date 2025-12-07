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

# run the following sciprts
bash scripts/2-deploy-token.sh
node scripts/3-add-token.js
node scripts/4-check-state.js
node scripts/5-add-proposer.js
node scripts/6-propose-mint.js
node scripts/8-execute-mint.js
node scripts/9-check-balance.js

# Creating token GGTNAp3YA3FxsvBxKPuZ4iqrGhB66H96k8Rx4xxm3E7z under program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
# Address:  GGTNAp3YA3FxsvBxKPuZ4iqrGhB66H96k8Rx4xxm3E7z
# Decimals:  9
```