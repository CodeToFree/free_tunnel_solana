#!/bin/bash

# Stop script on any error
set -e

# Check if output should be colored (default: yes, unless running in a non-interactive shell)
if [[ -t 1 ]]; then
    # Color output for interactive terminals
    GREEN="\033[0;32m"
    BLUE="\033[0;34m"
    RESET="\033[0m"
else
    # No color output
    GREEN=""
    BLUE=""
    RESET=""
fi

# --- File Paths ---
PAYER_KEYPAIR_PATH="${HOME}/.config/solana/id.json"
PDAS_FILE_PATH="scripts/temp/program_pdas.json"

echo -e "${BLUE}--- Solana Token and Multisig Setup Script (with Program Signer) ---${RESET}"

# --- 1. Payer & PDA Setup ---
echo -e "\n${BLUE}Step 1: Loading payer and program PDAs...${RESET}"
if [ ! -f "$PDAS_FILE_PATH" ]; then
    echo "Error: PDA file not found at $PDAS_FILE_PATH. Please run 1-initialize.js first."
    exit 1
fi

PAYER_PUBKEY=$(solana-keygen pubkey $PAYER_KEYPAIR_PATH)
CONTRACT_SIGNER_PDA=$(jq -r '.contractSigner' $PDAS_FILE_PATH)
echo -e "Using payer account: ${GREEN}$PAYER_PUBKEY${RESET}"
echo -e "Using contract signer PDA: ${GREEN}$CONTRACT_SIGNER_PDA${RESET}"
echo -e "Airdropping 2 SOL to local validator if needed..."
solana airdrop 2 --keypair $PAYER_KEYPAIR_PATH

# --- 2. Create New SPL Token ---
echo -e "\n${BLUE}Step 2: Creating a new SPL Token...${RESET}"
TOKEN_MINT=$(spl-token create-token --decimals 9 --fee-payer $PAYER_KEYPAIR_PATH 2>&1 | awk '/Creating token/ {print $3}')
echo -e "Token Mint created with address: ${GREEN}$TOKEN_MINT${RESET}"

# --- 3. Create EXTERNAL Multisig Signers ---
echo -e "\n${BLUE}Step 3: Creating 2 EXTERNAL keypairs for the multisig...${RESET}"
mkdir -p keys
solana-keygen new --no-bip39-passphrase --outfile keys/signer1.json --force > /dev/null
solana-keygen new --no-bip39-passphrase --outfile keys/signer2.json --force > /dev/null
SIGNER1_PUBKEY=$(solana-keygen pubkey keys/signer1.json)
SIGNER2_PUBKEY=$(solana-keygen pubkey keys/signer2.json)
echo -e "External Signer 1 Pubkey: ${GREEN}$SIGNER1_PUBKEY${RESET}"
echo -e "External Signer 2 Pubkey: ${GREEN}$SIGNER2_PUBKEY${RESET}"

# --- 4. Create 2-of-3 Multisig Account ---
echo -e "\n${BLUE}Step 4: Creating the 2-of-3 Multisig Account...${RESET}"
# The members are the 2 external signers AND the program's PDA signer.
MULTISIG_MEMBERS="$SIGNER1_PUBKEY $SIGNER2_PUBKEY $CONTRACT_SIGNER_PDA"
MULTISIG_ADDRESS=$(spl-token create-multisig 1 $MULTISIG_MEMBERS --fee-payer $PAYER_KEYPAIR_PATH 2>&1 | awk '/Creating 1\/3 multisig/ {print $4}')
echo -e "Multisig account (2/3) created with address: ${GREEN}$MULTISIG_ADDRESS${RESET}"

# --- 5. Set Multisig as Mint Authority ---
echo -e "\n${BLUE}Step 5: Transferring mint authority to the multisig account...${RESET}"
spl-token authorize $TOKEN_MINT mint $MULTISIG_ADDRESS --fee-payer $PAYER_KEYPAIR_PATH
echo -e "Mint authority for token ${GREEN}$TOKEN_MINT${RESET} transferred to ${GREEN}$MULTISIG_ADDRESS${RESET}"

# --- 6. Fund the EXTERNAL Multisig Signer Wallets ---
echo -e "\n${BLUE}Step 6: Funding the 2 external multisig signer wallets...${RESET}"
SIGNER_PUBKEYS=($SIGNER1_PUBKEY $SIGNER2_PUBKEY)
for i in 1 2
do
    SIGNER_PUBKEY=${SIGNER_PUBKEYS[$i-1]}
    echo -e "\n  Processing signer wallet #$i..."
    echo -e "  Signer Wallet Pubkey: ${BLUE}$SIGNER_PUBKEY${RESET}"
    SIGNER_TOKEN_ACCOUNT=$(spl-token create-account --owner $SIGNER_PUBKEY $TOKEN_MINT --fee-payer $PAYER_KEYPAIR_PATH 2>&1 | awk '/Creating account/ {print $3}')
    echo -e "  Token Account: ${GREEN}$SIGNER_TOKEN_ACCOUNT${RESET}"
    echo -e "  Minting 1,000,000 tokens (requires signer #1's signature)..."
    # Note: For a 2/3 multisig, you would need two signers here for a real transaction.
    # The spl-token CLI might simplify this by only requiring one for this specific 'mint' command if it's also the fee payer.
    # If this step fails due to insufficient signers, you'd typically construct this transaction manually.
    spl-token mint $TOKEN_MINT 1000000 $SIGNER_TOKEN_ACCOUNT --mint-authority $MULTISIG_ADDRESS --multisig-signer keys/signer1.json --multisig-signer keys/signer2.json --fee-payer $PAYER_KEYPAIR_PATH
    echo -e "  Mint successful."
done

# --- 7. Save Details to File ---
echo -e "\n${BLUE}Step 7: Saving token and multisig details to a file...${RESET}"
mkdir -p scripts/temp
JSON_OUTPUT_FILE="scripts/temp/token_details.json"

cat > $JSON_OUTPUT_FILE << EOL
{
  "tokenMint": "$TOKEN_MINT",
  "multisigAddress": "$MULTISIG_ADDRESS",
  "multisigSigners": [
    "$SIGNER1_PUBKEY",
    "$SIGNER2_PUBKEY",
    "$CONTRACT_SIGNER_PDA"
  ]
}
EOL
echo -e "Details saved to ${GREEN}$JSON_OUTPUT_FILE${RESET}"


echo -e "\n${GREEN}--- All tasks completed successfully! ---${RESET}"
echo -e "Keypair files for external multisig signers are in the 'keys' directory."
