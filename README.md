# Free Tunnel Solana

## Local Chain Testing

```bash
# build the program
cargo build-sbf

# set the url to localhost
solana config set --url localhost

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

## Mainnet/Testnet/Devnet Testing

```bash
# set the url
solana config set --url https://api.devnet.solana.com

# check balance (we need at least 3 $SOL to deploy)
solana balance

# deploy the program
solana program deploy target/deploy/free_tunnel_solana.so

# fill in the `SOL_RPC` in the `.env` file
# example: SOL_RPC="https://api.devnet.solana.com"

# backup the keypair in `./target/deploy/free_tunnel_solana-keypair.json`,
#  that's the program keypair.

# initialize the program
node scripts/devnet-initialize.js

# deploy & add the token
node scripts/devnet-deploy-add-token.js

# add proposer
node scripts/devnet-add-proposer.js

# remove token
node scripts/devnet-remove-token.js
```