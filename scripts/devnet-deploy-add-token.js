import {
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  createMint,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
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
const RPC_URL = process.env.SOL_RPC || "https://api.devnet.solana.com";
const PDAS_FILE_PATH = path.join("scripts", "temp", "program_pdas.json");

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

  async function createAndRegisterToken(name, decimals, index, tokenProgramId) {
    console.log(`\n--- Processing ${name} (Index: ${index}) ---`);
    
    // 1. Create Mint (Authority = Admin)
    console.log(`Creating Mint...`);
    const mint = await createMint(
      connection,
      admin,
      admin.publicKey, // Mint Authority = Admin
      null, // Freeze Authority
      decimals,
      undefined,
      undefined,
      tokenProgramId
    );
    console.log(`Mint Created: ${BLUE}${mint.toBase58()}${RESET}`);

    // 2. AddToken Instruction
    console.log(`Registering with Bridge...`);
    const tokenAccountContract = await getAssociatedTokenAddress(
      mint,
      contractSignerPda,
      true,
      tokenProgramId,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const payload = { token_index: index };
    const payloadBuffer = borsh.serialize(INSTRUCTION_SCHEMA, payload);
    const instructionBuffer = Buffer.concat([Buffer.from([5]), payloadBuffer]); // 5 = AddToken

    const ix = new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        { pubkey: admin.publicKey, isSigner: true, isWritable: true },
        { pubkey: tokenAccountContract, isSigner: false, isWritable: true },
        { pubkey: contractSignerPda, isSigner: false, isWritable: false },
        { pubkey: basicStoragePda, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
        { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data: instructionBuffer,
    });

    const signature = await sendAndConfirmTransaction(connection, new Transaction().add(ix), [admin]);
    console.log(`${GREEN}Success! Token Registered. Tx: ${signature}${RESET}`);
    return mint.toBase58();
  }

  // Task 3: SolvBTC (Standard, Decimals 8, Index 78)
  const solvBtc = await createAndRegisterToken("SolvBTC", 8, 78, TOKEN_PROGRAM_ID);

  // Task 4: xSolvBTC (Token-2022, Decimals 8, Index 79)
  const xSolvBtc = await createAndRegisterToken("xSolvBTC", 8, 79, TOKEN_2022_PROGRAM_ID);

  console.log("\n--- Summary ---");
  console.log(`SolvBTC: ${solvBtc}`);
  console.log(`xSolvBTC: ${xSolvBtc}`);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});

