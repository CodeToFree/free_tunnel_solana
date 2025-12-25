import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as borsh from "borsh";
import fs from "fs";
import path from "path";
import dotenv from "dotenv";
import { loadProgramKeypair, loadAdminKeypair } from "./utils.js";

dotenv.config();

const { programId: PROGRAM_ID } = loadProgramKeypair();
const RPC_URL = process.env.SOL_RPC || "http://127.0.0.1:8899";
const PDAS_FILE_PATH = path.join("scripts", "temp", "program_pdas.json");

const INSTRUCTION_SCHEMA = {
  struct: {
    new_proposer: { array: { type: 'u8', len: 32 } },
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
  
  const PROPOSER_PUBKEY_STR = "HsDJ6bKrtKLeFjgBZqKvc2iGDGgGyvXCLEubfVCaZ4PQ";
  const newProposer = new PublicKey(PROPOSER_PUBKEY_STR);

  console.log(`Adding Proposer: ${BLUE}${newProposer.toBase58()}${RESET}`);

  const payload = { new_proposer: newProposer.toBuffer() };
  const payloadBuffer = borsh.serialize(INSTRUCTION_SCHEMA, payload);
  const instructionBuffer = Buffer.concat([Buffer.from([2]), payloadBuffer]); // 2 = AddProposer

  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: false },
      { pubkey: new PublicKey(pdas.basicStorage), isSigner: false, isWritable: true },
    ],
    data: instructionBuffer,
  });

  const tx = new Transaction().add(ix);
  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, tx, [admin]);
  console.log(`\n${GREEN}Success! Transaction: ${signature}${RESET}`);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});

