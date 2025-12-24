import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
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

// --- Configuration ---
const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";

const { programId: PROGRAM_ID } = loadProgramKeypair();
console.log(`Program ID: ${BLUE}${PROGRAM_ID.toBase58()}${RESET}`);
const RPC_URL = process.env.SOL_RPC || "https://api.devnet.solana.com";

// Setup storage for PDAs to share with other scripts
const TEMP_DIR = path.join("scripts", "temp");
const PDAS_FILE_PATH = path.join(TEMP_DIR, "program_pdas.json");

const INSTRUCTION_SCHEMA = {
  struct: {
    is_mint_contract: 'u8',
    executors: { array: { type: { array: { type: 'u8', len: 20 } } } },
    threshold: 'u64',
    exe_index: 'u64'
  }
};

function savePdasToFile(pdas) {
  if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
  }
  fs.writeFileSync(PDAS_FILE_PATH, JSON.stringify(pdas, null, 2));
  console.log(`\n${GREEN}Successfully saved PDA addresses to ${BLUE}${PDAS_FILE_PATH}${RESET}`);
}

async function main() {
  console.log("\nConnecting to Devnet...");
  const connection = new Connection(RPC_URL, "confirmed");

  const admin = loadAdminKeypair();
  console.log(`Using Admin: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);

  const isMintContract = true;
  const executorsHex = [
    "0014Eb4Ac6Dd1473b258d088E6EF214b2BCdc53C",
    "9E498DD03c5E984C105E83221AA911DEC4844dB5",
    "32369C32113D6A85d4B71faA40DDd048187DCe79",
    "cd6d31668524598755b81A2cee068Ae2ea6979b9"
  ];
  const executors = executorsHex.map(hex => Buffer.from(hex, 'hex'));
  const threshold = BigInt(3);
  const exeIndex = BigInt(0);

  // Calculate PDAs
  console.log("\nCalculating PDA addresses...");
  const [basicStoragePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("basic-storage")],
    PROGRAM_ID
  );

  const exeIndexBuffer = Buffer.alloc(8);
  exeIndexBuffer.writeBigUInt64LE(exeIndex);
  const [executorsAtIndexPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("executors"), exeIndexBuffer],
    PROGRAM_ID
  );

  const [contractSignerPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("contract-signer")],
    PROGRAM_ID
  );

  const pdas = {
    basicStorage: basicStoragePda.toBase58(),
    executorsAtIndex0: executorsAtIndexPda.toBase58(),
    contractSigner: contractSignerPda.toBase58(),
  };

  console.log(`PDA [Basic Storage]: ${pdas.basicStorage}`);
  console.log(`PDA [Executors #0]: ${pdas.executorsAtIndex0}`);
  console.log(`PDA [Contract Signer]: ${pdas.contractSigner}`);
  
  savePdasToFile(pdas);

  // Serialize Instruction
  const instructionDataPayload = {
    is_mint_contract: isMintContract ? 1 : 0,
    executors: executors,
    threshold: threshold,
    exe_index: exeIndex,
  };

  const payloadBuffer = borsh.serialize(INSTRUCTION_SCHEMA, instructionDataPayload);
  const instructionBuffer = Buffer.concat([Buffer.from([0]), payloadBuffer]); // 0 = Initialize

  // Create Transaction
  const ix = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: basicStoragePda, isSigner: false, isWritable: true },
      { pubkey: executorsAtIndexPda, isSigner: false, isWritable: true },
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

