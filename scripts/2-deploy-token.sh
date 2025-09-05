#!/bin/bash

# Stop script on any error
set -e

# Check if output should be colored (default: yes, unless NO_COLOR is set)
if [[ "${NO_COLOR:-}" == "1" || "${NO_COLOR:-}" == "true" || ! -t 1 ]]; then
    # No color output
    GREEN=""
    BLUE=""
    RESET=""
else
    # Color output
    GREEN="\033[0;32m"
    BLUE="\033[0;34m"
    RESET="\033[0m"
fi

echo -e "${BLUE}--- Solana Token and Multisig Setup Script ---${RESET}"

# --- 1. Payer Setup ---
echo -e "\n${BLUE}Step 1: Checking payer account...${RESET}"
# Define the keypair path for the default CLI wallet to be used as the fee payer.
PAYER_KEYPAIR_PATH="${HOME}/.config/solana/id.json"
PAYER_PUBKEY=$(solana-keygen pubkey $PAYER_KEYPAIR_PATH)
echo -e "Using payer keypair at: ${BLUE}$PAYER_KEYPAIR_PATH${RESET}"
echo -e "Using payer account: ${GREEN}$PAYER_PUBKEY${RESET}"
echo "Airdropping 2 SOL to local validator if needed..."
solana airdrop 2 $PAYER_PUBKEY

# --- 2. Create New SPL Token ---
echo -e "\n${BLUE}Step 2: Creating a new SPL Token...${RESET}"
# Redirect stderr to stdout (2>&1) so that awk can capture the progress message
TOKEN_MINT_OUTPUT=$(spl-token create-token --decimals 9 --fee-payer $PAYER_KEYPAIR_PATH 2>&1)
TOKEN_MINT=$(echo "$TOKEN_MINT_OUTPUT" | awk '/Creating token/ {print $3}')
echo "$TOKEN_MINT_OUTPUT"
echo -e "Token Mint created with address: ${GREEN}$TOKEN_MINT${RESET}"

# --- 3. Create Multisig Signers ---
echo -e "\n${BLUE}Step 3: Creating 3 keypairs for the multisig...${RESET}"
mkdir -p keys
solana-keygen new --no-bip39-passphrase --outfile keys/signer1.json --force > /dev/null
solana-keygen new --no-bip39-passphrase --outfile keys/signer2.json --force > /dev/null
solana-keygen new --no-bip39-passphrase --outfile keys/signer3.json --force > /dev/null
SIGNER1_PUBKEY=$(solana-keygen pubkey keys/signer1.json)
SIGNER2_PUBKEY=$(solana-keygen pubkey keys/signer2.json)
SIGNER3_PUBKEY=$(solana-keygen pubkey keys/signer3.json)
echo -e "Signer 1 Pubkey: ${GREEN}$SIGNER1_PUBKEY${RESET}"
echo -e "Signer 2 Pubkey: ${GREEN}$SIGNER2_PUBKEY${RESET}"
echo -e "Signer 3 Pubkey: ${GREEN}$SIGNER3_PUBKEY${RESET}"

# --- 4. Create 1-of-3 Multisig Account ---
echo -e "\n${BLUE}Step 4: Creating the 1-of-3 Multisig Account...${RESET}"
# The address is the 4th field in the output for this specific command
MULTISIG_OUTPUT=$(spl-token create-multisig 1 $SIGNER1_PUBKEY $SIGNER2_PUBKEY $SIGNER3_PUBKEY --fee-payer $PAYER_KEYPAIR_PATH 2>&1)
MULTISIG_ADDRESS=$(echo "$MULTISIG_OUTPUT" | awk '/Creating 1\/3 multisig/ {print $4}')
echo "$MULTISIG_OUTPUT"
echo -e "Multisig account created with address: ${GREEN}$MULTISIG_ADDRESS${RESET}"

# --- 5. Set Multisig as Mint Authority ---
echo -e "\n${BLUE}Step 5: Transferring mint authority to the multisig account...${RESET}"
spl-token authorize $TOKEN_MINT mint $MULTISIG_ADDRESS --fee-payer $PAYER_KEYPAIR_PATH
echo -e "Mint authority for token ${GREEN}$TOKEN_MINT${RESET} transferred to ${GREEN}$MULTISIG_ADDRESS${RESET}"

# --- 6. Fund the Multisig Signer Wallets ---
echo -e "\n${BLUE}Step 6: Funding the 3 multisig signer wallets...${RESET}"

SIGNER_PUBKEYS=($SIGNER1_PUBKEY $SIGNER2_PUBKEY $SIGNER3_PUBKEY)

for i in 1 2 3
do
    SIGNER_PUBKEY=${SIGNER_PUBKEYS[$i-1]}
    echo -e "\n  Processing signer wallet #$i..."
    echo -e "  Signer Wallet Pubkey: ${BLUE}$SIGNER_PUBKEY${RESET}"

    # Create the associated token account for this signer wallet
    ACCOUNT_OUTPUT=$(spl-token create-account --owner $SIGNER_PUBKEY $TOKEN_MINT --fee-payer $PAYER_KEYPAIR_PATH 2>&1)
    SIGNER_TOKEN_ACCOUNT=$(echo "$ACCOUNT_OUTPUT" | awk '/Creating account/ {print $3}')
    echo "$ACCOUNT_OUTPUT"
    echo -e "  Token Account: ${GREEN}$SIGNER_TOKEN_ACCOUNT${RESET}"

    # Mint 1,000,000 tokens.
    echo "  Minting 1,000,000 tokens..."
    spl-token mint $TOKEN_MINT 1000000 $SIGNER_TOKEN_ACCOUNT --mint-authority $MULTISIG_ADDRESS --multisig-signer keys/signer1.json --fee-payer $PAYER_KEYPAIR_PATH
    echo "  Mint successful."
done

echo -e "\n${GREEN}--- All tasks completed successfully! ---${RESET}"
echo -e "Keypair files for multisig signers are stored in the 'keys' directory: ${BLUE}keys/signer1.json, keys/signer2.json, keys/signer3.json${RESET}"

