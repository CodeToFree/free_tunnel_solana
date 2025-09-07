import { Connection, PublicKey } from "@solana/web3.js";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import fs from "fs";
import path from "path";

// --- Configuration ---
const RPC_URL = "http://127.0.0.1:8899";
const TEMP_DIR = path.join("scripts", "temp");
const TOKEN_DETAILS_PATH = path.join(TEMP_DIR, "token_details.json");
const PROPOSAL_DETAILS_PATH = path.join(TEMP_DIR, "proposal_details.json");

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

/**
 * Loads and parses a JSON file.
 * @param {string} filePath - The path to the JSON file.
 * @returns {object} The parsed JSON object.
 */
function loadJsonFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`File not found: ${filePath}. Please run the previous scripts first.`);
  }
  return JSON.parse(fs.readFileSync(filePath, 'utf-8'));
}

async function main() {
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading details from temp files...");
  const tokenDetails = loadJsonFile(TOKEN_DETAILS_PATH);
  const proposalDetails = loadJsonFile(PROPOSAL_DETAILS_PATH);

  const recipient = new PublicKey(proposalDetails.recipient);
  const tokenMint = new PublicKey(tokenDetails.tokenMint);

  console.log(`\nChecking balance for:`);
  console.log(`   ${BLUE}Recipient Wallet:${RESET} ${recipient.toBase58()}`);
  console.log(`   ${BLUE}Token Mint:${RESET}       ${tokenMint.toBase58()}`);

  // Calculate the Associated Token Account address
  const recipientTokenAccount = await getAssociatedTokenAddress(tokenMint, recipient);
  console.log(`   ${BLUE}Token Account:${RESET}    ${recipientTokenAccount.toBase58()}`);

  try {
    const balanceResponse = await connection.getTokenAccountBalance(recipientTokenAccount);

    if (balanceResponse.value.uiAmountString) {
      console.log(`\n--- ${GREEN}Verification Success!${RESET} ---`);
      console.log(`Recipient's token balance is: ${YELLOW}${balanceResponse.value.uiAmountString}${RESET}`);
    } else {
      console.log(`\n--- ${YELLOW}Verification Note${RESET} ---`);
      console.log("Recipient has a token account, but the balance is 0.");
    }
  } catch (error) {
    if (error.message.includes("could not find account")) {
      console.log(`\n--- ${YELLOW}Verification Failed${RESET} ---`);
      console.log("Recipient's token account does not exist or has been closed.");
    } else {
      throw error; // Re-throw other unexpected errors
    }
  }
}

main().catch((error) => {
  console.error("An error occurred:", error);
  process.exit(1);
});

