import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as borsh from "borsh";
import fs from "fs";
import os from "os";
import path from "path";

// --- Configuration ---
const PROGRAM_ID = new PublicKey(
  "4y5qquCkpjqpMvkivnk7DYxekuX5ApKqcn4uFarjJVrj"
);

const RPC_URL = "http://127.0.0.1:8899";

// Borsh schema for the AddProposer instruction
const INSTRUCTION_SCHEMA = {
  struct: {
    new_proposer: { array: { type: 'u8', len: 32 } },
  }
};

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

/**
 * Loads the default Solana CLI keypair to act as the admin/payer.
 * @returns {Keypair} The keypair loaded from the default path.
 */
function loadAdminKeypair() {
  const keypairPath = path.join(os.homedir(), '.config', 'solana', 'id.json');
  if (!fs.existsSync(keypairPath)) {
    throw new Error("Could not find Solana CLI keypair at default path. Please ensure it exists.");
  }
  const secretKey = JSON.parse(fs.readFileSync(keypairPath, 'utf-8'));
  return Keypair.fromSecretKey(new Uint8Array(secretKey));
}


async function main() {
  // 1. Setup accounts
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading admin/payer account from default Solana CLI path...");
  const admin = loadAdminKeypair();
  console.log(`Using Admin account: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);
  console.log(`This admin account will also be added as the new proposer.`);

  // 2. Calculate PDA addresses
  console.log("\nCalculating PDA addresses...");

  const [basicStoragePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("basic-storage")],
    PROGRAM_ID
  );

  console.log(`PDA ${BLUE}[Basic Storage]${RESET}: ${basicStoragePda.toBase58()}`);

  // 3. Serialize instruction data
  const instructionDataPayload = {
    new_proposer: admin.publicKey.toBuffer(),
  };

  const payloadBuffer = borsh.serialize(
    INSTRUCTION_SCHEMA,
    instructionDataPayload
  );

  // Prepend the instruction index (2 for AddProposer)
  const instructionBuffer = Buffer.concat([
    Buffer.from([2]),
    payloadBuffer
  ]);

  // 4. Create and Send Transaction
  console.log("\nCreating AddProposer instruction...");
  const addProposerInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      // 0. account_admin
      { pubkey: admin.publicKey, isSigner: true, isWritable: false },
      // 1. data_account_basic_storage
      { pubkey: basicStoragePda, isSigner: false, isWritable: true },
    ],
    data: instructionBuffer,
  });

  const transaction = new Transaction().add(addProposerInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [
    admin,
  ]);

  console.log("\n--- Success! ---");
  console.log(`Transaction Signature: ${signature}`);
  console.log(`Admin ${YELLOW}${admin.publicKey.toBase58()}${RESET} has been added as a proposer.`);
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
