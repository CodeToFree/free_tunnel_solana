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
const REQ_ID_PATH = path.join("scripts", "temp", "reqid.bin");

// Borsh schema for the CancelMint instruction's data
const INSTRUCTION_SCHEMA = {
  struct: {
    req_id: { array: { type: 'u8', len: 32 } },
  }
};

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";

/**
 * Loads the default Solana CLI keypair to act as the admin/payer/proposer.
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

/**
 * Loads the previously saved ReqId from a binary file.
 * @returns {Buffer} The 32-byte ReqId buffer.
 */
function loadReqId() {
  if (!fs.existsSync(REQ_ID_PATH)) {
    throw new Error(`Could not find ReqId file at ${REQ_ID_PATH}. Please run 6-propose-mint.js first.`);
  }
  return fs.readFileSync(REQ_ID_PATH);
}


async function main() {
  // 1. Setup accounts and load data
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading proposer/payer account from default Solana CLI path...");
  const proposer = loadAdminKeypair();
  console.log(`Using Proposer account: ${BLUE}${proposer.publicKey.toBase58()}${RESET}`);

  console.log(`Loading ReqId from: ${BLUE}${REQ_ID_PATH}${RESET}`);
  const reqId = loadReqId();
  console.log(`ReqId (hex): ${GREEN}${reqId.toString('hex')}${RESET}`);

  // 2. Calculate PDA addresses
  console.log("\nCalculating PDA addresses...");

  const [basicStoragePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("basic-storage")],
    PROGRAM_ID
  );

  const [proposedMintPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("mint"), reqId],
    PROGRAM_ID
  );

  console.log(`PDA ${BLUE}[Basic Storage]${RESET}: ${basicStoragePda.toBase58()}`);
  console.log(`PDA ${BLUE}[Proposed Mint]${RESET}: ${proposedMintPda.toBase58()}`);

  // 3. Serialize instruction data
  const instructionDataPayload = {
    req_id: reqId,
  };

  const payloadBuffer = borsh.serialize(
    INSTRUCTION_SCHEMA,
    instructionDataPayload
  );

  // Prepend the instruction index (9 for CancelMint)
  const instructionBuffer = Buffer.concat([
    Buffer.from([9]),
    payloadBuffer
  ]);

  // 4. Create and Send Transaction
  console.log("\nCreating CancelMint instruction...");
  const cancelMintInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      // 0. data_account_basic_storage
      { pubkey: basicStoragePda, isSigner: false, isWritable: false },
      // 1. data_account_proposed_mint
      { pubkey: proposedMintPda, isSigner: false, isWritable: true },
      // 2. account_refund
      { pubkey: proposer.publicKey, isSigner: false, isWritable: true },
    ],
    data: instructionBuffer,
  });

  // Note: CancelMint might be permissionless after a timeout, 
  // but for testing, we'll have the proposer sign to also pay for the transaction.
  const transaction = new Transaction().add(cancelMintInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [
    proposer,
  ]);

  console.log("\n--- Success! ---");
  console.log(`Transaction Signature: ${signature}`);
  console.log(`Mint proposal for ReqId ${GREEN}${reqId.toString('hex').substring(0, 16)}...${RESET} has been cancelled.`);
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
