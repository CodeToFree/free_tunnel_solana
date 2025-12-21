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

// Schema for SparseArray items
const SPARSE_ARRAY_ITEM_PUBKEY = { struct: { id: 'u8', value: PUBKEY_SCHEMA } };
const SPARSE_ARRAY_ITEM_U8 = { struct: { id: 'u8', value: 'u8' } };
const SPARSE_ARRAY_ITEM_U64 = { struct: { id: 'u8', value: 'u64' } };

// This schema must EXACTLY match your Rust structs in `state.rs`
// BasicStorage struct: mint_or_lock, admin, proposers, executors_group_length,
// tokens, vaults, decimals, locked_balance
const BASIC_STORAGE_SCHEMA = {
  struct: {
    // mint_or_lock: bool
    mint_or_lock: 'u8', // Borsh serializes bool as u8
    // admin: Pubkey
    admin: PUBKEY_SCHEMA,
    // proposers: Vec<Pubkey>
    proposers: { array: { type: PUBKEY_SCHEMA } },
    // executors_group_length: u64
    executors_group_length: 'u64',
    // tokens: SparseArray<Pubkey>
    tokens: { array: { type: SPARSE_ARRAY_ITEM_PUBKEY } },
    // vaults: SparseArray<Pubkey>
    vaults: { array: { type: SPARSE_ARRAY_ITEM_PUBKEY } },
    // decimals: SparseArray<u8>
    decimals: { array: { type: SPARSE_ARRAY_ITEM_U8 } },
    // locked_balance: SparseArray<u64>
    locked_balance: { array: { type: SPARSE_ARRAY_ITEM_U64 } },
  }
};


const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

async function main() {
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Calculating PDA address for Basic Storage account...");
  const [basicStoragePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("basic-storage")],
    PROGRAM_ID
  );
  console.log(`Querying account: ${BLUE}${basicStoragePda.toBase58()}${RESET}`);

  // 1. Fetch the account info
  const accountInfo = await connection.getAccountInfo(basicStoragePda);

  if (!accountInfo) {
    console.error("Error: Basic Storage account not found. Has it been initialized?");
    return;
  }

  // 2. Decode the account data
  console.log("\nDecoding account data...");

  // Your program uses a 4-byte little-endian u32 for the length prefix
  const dataLength = accountInfo.data.readUInt32LE(0);
  const dataBuffer = accountInfo.data.subarray(4, 4 + dataLength);

  const decodedData = borsh.deserialize(
    BASIC_STORAGE_SCHEMA,
    dataBuffer
  );

  console.log(`${GREEN}--- Decoded BasicStorage State ---${RESET}`);
  console.log(`Contract Type: ${decodedData.mint_or_lock ? 'Mint' : 'Lock'}`);
  console.log(`Admin: ${new PublicKey(decodedData.admin).toBase58()}`);
  console.log(`Proposers:`, decodedData.proposers.map(p => new PublicKey(p).toBase58()));
  console.log(`Executors Group Length: ${decodedData.executors_group_length}`);
  console.log(`Tokens:`, decodedData.tokens);
  console.log(`Vaults:`, decodedData.vaults);
  console.log(`Decimals:`, decodedData.decimals);
  console.log(`Locked Balance:`, decodedData.locked_balance);

  // 3. Verify the specific token at index 56
  console.log(`\n${YELLOW}--- Verifying Token at Index 56 ---${RESET}`);

  const targetIndex = 56;

  const tokenEntry = decodedData.tokens.find(t => t.id === targetIndex);
  const vaultEntry = decodedData.vaults.find(v => v.id === targetIndex);
  const decimalsEntry = decodedData.decimals.find(d => d.id === targetIndex);
  const balanceEntry = decodedData.locked_balance.find(b => b.id === targetIndex);

  if (tokenEntry) {
    const tokenMint = new PublicKey(tokenEntry.value).toBase58();
    console.log(`Token Mint at index ${targetIndex}: ${GREEN}${tokenMint}${RESET}`);

    if (vaultEntry) {
      const vault = new PublicKey(vaultEntry.value).toBase58();
      console.log(`Vault ATA at index ${targetIndex}: ${GREEN}${vault}${RESET}`);
    } else {
      console.error(`Error: Vault for index ${targetIndex} not found!`);
    }

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
