import { Connection, PublicKey } from "@solana/web3.js";
import * as borsh from "borsh";

// --- Configuration ---
const PROGRAM_ID = new PublicKey(
  "4y5qquCkpjqpMvkivnk7DYxekuX5ApKqcn4uFarjJVrj"
);
const RPC_URL = "http://127.0.0.1:8899";

// --- Borsh Schemas to decode on-chain data ---

// A helper schema for a Pubkey (32-byte array)
const PUBKEY_SCHEMA = { array: { type: 'u8', len: 32 } };

// This schema must EXACTLY match your Rust structs in `state.rs`
const TOKENS_AND_PROPOSERS_SCHEMA = {
  struct: {
    // For `tokens: SparseArray<Pubkey>`
    tokens: { array: { type: { struct: { id: 'u8', value: PUBKEY_SCHEMA } } } },
    // For `decimals: SparseArray<u8>`
    decimals: { array: { type: { struct: { id: 'u8', value: 'u8' } } } },
    // For `locked_balance: SparseArray<u64>`
    locked_balance: { array: { type: { struct: { id: 'u8', value: 'u64' } } } },
    // For `proposers: Vec<Pubkey>`
    proposers: { array: { type: PUBKEY_SCHEMA } }
  }
};


const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

async function main() {
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Calculating PDA address for Tokens/Proposers account...");
  const [tokensProposersPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("tokens-proposers")],
    PROGRAM_ID
  );
  console.log(`Querying account: ${BLUE}${tokensProposersPda.toBase58()}${RESET}`);

  // 1. Fetch the account info
  const accountInfo = await connection.getAccountInfo(tokensProposersPda);

  if (!accountInfo) {
    console.error("Error: Tokens/Proposers account not found. Has it been initialized?");
    return;
  }

  // 2. Decode the account data
  console.log("\nDecoding account data...");

  // Your program uses a 4-byte little-endian u32 for the length prefix
  const dataLength = accountInfo.data.readUInt32LE(0);
  const dataBuffer = accountInfo.data.slice(4, 4 + dataLength);

  const decodedData = borsh.deserialize(
    TOKENS_AND_PROPOSERS_SCHEMA,
    dataBuffer
  );

  console.log(`${GREEN}--- Decoded TokensAndProposers State ---${RESET}`);
  console.log(decodedData);

  // 3. Verify the specific token at index 56
  console.log(`\n${YELLOW}--- Verifying Token at Index 56 ---${RESET}`);

  const targetIndex = 56;

  const tokenEntry = decodedData.tokens.find(t => t.id === targetIndex);
  const decimalsEntry = decodedData.decimals.find(d => d.id === targetIndex);
  const balanceEntry = decodedData.locked_balance.find(b => b.id === targetIndex);

  if (tokenEntry) {
    const tokenMint = new PublicKey(tokenEntry.value).toBase58();
    console.log(`Token Mint at index ${targetIndex}: ${GREEN}${tokenMint}${RESET}`);

    if (decimalsEntry) {
      console.log(`Decimals at index ${targetIndex}:   ${GREEN}${decimalsEntry.value}${RESET}`);
    } else {
      console.error(`Error: Decimals for index ${targetIndex} not found!`);
    }

    if (balanceEntry) {
      console.log(`Locked Balance at index ${targetIndex}: ${GREEN}${balanceEntry.value}${RESET}`);
    } else {
      console.error(`Error: Locked Balance for index ${targetIndex} not found!`);
    }

  } else {
    console.error(`Error: Token with index ${targetIndex} was not found in the account data.`);
  }

  console.log("\n--- Verification Complete ---");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
