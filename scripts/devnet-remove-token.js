import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import * as borsh from "borsh";
import fs from "fs";
import path from "path";
import dotenv from "dotenv";
import { loadProgramKeypair, loadAdminKeypair } from "./utils.js";

dotenv.config();

const { programId: PROGRAM_ID } = loadProgramKeypair();
const RPC_URL = process.env.SOL_RPC || "http://127.0.0.1:8899";
const PDAS_FILE_PATH = path.join("scripts", "temp", "program_pdas.json");

// --- Borsh Schemas (adapted from check-state.js) ---
const PUBKEY_SCHEMA = { array: { type: 'u8', len: 32 } };
const SPARSE_ARRAY_ITEM_PUBKEY = { struct: { id: 'u8', value: PUBKEY_SCHEMA } };
const SPARSE_ARRAY_ITEM_U8 = { struct: { id: 'u8', value: 'u8' } };
const SPARSE_ARRAY_ITEM_U64 = { struct: { id: 'u8', value: 'u64' } };

const BASIC_STORAGE_SCHEMA = {
  struct: {
    mint_or_lock: 'u8',
    admin: PUBKEY_SCHEMA,
    proposers: { array: { type: PUBKEY_SCHEMA } },
    executors_group_length: 'u64',
    tokens: { array: { type: SPARSE_ARRAY_ITEM_PUBKEY } },
    vaults: { array: { type: SPARSE_ARRAY_ITEM_PUBKEY } },
    decimals: { array: { type: SPARSE_ARRAY_ITEM_U8 } },
    locked_balance: { array: { type: SPARSE_ARRAY_ITEM_U64 } },
  }
};

const INSTRUCTION_SCHEMA = {
  struct: {
    token_index: 'u8',
  }
};

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";

function loadPdas() {
  if (!fs.existsSync(PDAS_FILE_PATH)) throw new Error("PDAs not found. Run initialize first.");
  return JSON.parse(fs.readFileSync(PDAS_FILE_PATH, 'utf-8'));
}

async function main() {
  console.log("\nConnecting to Devnet...");
  const connection = new Connection(RPC_URL, "confirmed");
  const admin = loadAdminKeypair();
  const pdas = loadPdas();
  const contractSignerPda = new PublicKey(pdas.contractSigner);
  const basicStoragePda = new PublicKey(pdas.basicStorage);

  async function getMintFromStorage(index) {
    const accountInfo = await connection.getAccountInfo(basicStoragePda);
    if (!accountInfo) throw new Error("BasicStorage account not found");
    
    // Decoding using Borsh schema as in 4-check-state.js
    try {
        // Your program uses a 4-byte little-endian u32 for the length prefix
        const dataLength = accountInfo.data.readUInt32LE(0);
        const dataBuffer = accountInfo.data.subarray(4, 4 + dataLength);
    
        const decodedData = borsh.deserialize(
          BASIC_STORAGE_SCHEMA,
          dataBuffer
        );
        
        const tokenEntry = decodedData.tokens.find(t => t.id === index);
        if (tokenEntry) {
            return new PublicKey(tokenEntry.value);
        }
    } catch (e) {
        console.error("Error decoding storage:", e);
        throw e;
    }

    return null;
  }

  async function removeToken(index, tokenProgramId) {
    console.log(`\nLooking for token at index ${index}...`);
    const mint = await getMintFromStorage(index);
    
    if (!mint) {
      console.log(`Token at index ${index} not found in storage. Skipping.`);
      return;
    }
    
    console.log(`Found Mint: ${BLUE}${mint.toBase58()}${RESET}`);

    // Calculate Token Account (Vault)
    const tokenAccountContract = await getAssociatedTokenAddress(
      mint,
      contractSignerPda,
      true,
      tokenProgramId,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );
    console.log(`Contract signer: ${BLUE}${contractSignerPda.toBase58()}${RESET}`);
    console.log(`Token account for contract: ${BLUE}${tokenAccountContract.toBase58()}${RESET}`);
    console.log(`Removing...`);

    const payload = { token_index: index };
    const payloadBuffer = borsh.serialize(INSTRUCTION_SCHEMA, payload);
    const instructionBuffer = Buffer.concat([Buffer.from([6]), payloadBuffer]); // 6 = RemoveToken

    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: basicStoragePda, isSigner: false, isWritable: true },
        { pubkey: tokenAccountContract, isSigner: false, isWritable: false },
      ],
      data: instructionBuffer,
    });

    try {
      const signature = await sendAndConfirmTransaction(connection, new Transaction().add(ix), [admin]);
      console.log(`${GREEN}Success! Token Index ${index} Removed. Tx: ${signature}${RESET}`);
    } catch (err) {
      console.error(`Failed to remove token index ${index}:`, err);
    }
  }

  const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
  const TOKEN_2022_PROGRAM_ID = new PublicKey("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
  
  await removeToken(78, TOKEN_PROGRAM_ID);
  await removeToken(79, TOKEN_2022_PROGRAM_ID);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
