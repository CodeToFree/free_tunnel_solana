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
const TOKEN_DETAILS_PATH = path.join("scripts", "temp", "token_details.json");
const RPC_URL = "http://127.0.0.1:8899";

// --- Instruction Data ---
const TOKEN_TO_ADD = {
  index: 56,
  // We will load the mint address from the file created by 2-deploy-token.sh
  decimals: 9, // This matches the decimals set in 2-deploy-token.sh
};

// Borsh schema for the AddToken instruction
const INSTRUCTION_SCHEMA = {
  struct: {
    token_index: 'u8',
    token_pubkey: { array: { type: 'u8', len: 32 } },
    token_decimals: 'u8',
  }
};

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";

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

/**
 * Loads token details from the generated JSON file.
 * @returns {{tokenMint: string, multisigAddress: string, multisigSigners: string[]}}
 */
function loadTokenDetails() {
    if (!fs.existsSync(TOKEN_DETAILS_PATH)) {
        throw new Error(`Could not find token details file at ${TOKEN_DETAILS_PATH}. Please run 2-deploy-token.sh first.`);
    }
    return JSON.parse(fs.readFileSync(TOKEN_DETAILS_PATH, 'utf-8'));
}


async function main() {
  // 1. Setup accounts
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading admin/payer account from default Solana CLI path...");
  const admin = loadAdminKeypair();
  console.log(`Using Admin account: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);

  console.log(`Loading token details from ${BLUE}${TOKEN_DETAILS_PATH}${RESET}...`);
  const tokenDetails = loadTokenDetails();
  const TOKEN_MINT = new PublicKey(tokenDetails.tokenMint);
  console.log(`Token Mint to add: ${GREEN}${TOKEN_MINT.toBase58()}${RESET}`);


  // 2. Calculate PDA addresses
  console.log("\nCalculating PDA addresses...");

  const [basicStoragePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("basic-storage")],
    PROGRAM_ID
  );

  const [tokensProposersPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("tokens-proposers")],
    PROGRAM_ID
  );

  console.log(`PDA ${BLUE}[Basic Storage]${RESET}: ${basicStoragePda.toBase58()}`);
  console.log(`PDA ${BLUE}[Tokens/Proposers]${RESET}: ${tokensProposersPda.toBase58()}`);

  // 3. Serialize instruction data
  const instructionDataPayload = {
    token_index: TOKEN_TO_ADD.index,
    token_pubkey: TOKEN_MINT.toBuffer(),
    token_decimals: TOKEN_TO_ADD.decimals,
  };

  const payloadBuffer = borsh.serialize(
    INSTRUCTION_SCHEMA,
    instructionDataPayload
  );

  // Prepend the instruction index (5 for AddToken)
  const instructionBuffer = Buffer.concat([
    Buffer.from([5]),
    payloadBuffer
  ]);

  // 4. Create and Send Transaction
  console.log("\nCreating AddToken instruction...");
  const addTokenInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      // 0. account_admin
      { pubkey: admin.publicKey, isSigner: true, isWritable: false },
      // 1. data_account_basic_storage
      { pubkey: basicStoragePda, isSigner: false, isWritable: false },
      // 2. data_account_tokens_proposers
      { pubkey: tokensProposersPda, isSigner: false, isWritable: true },
    ],
    data: instructionBuffer,
  });

  const transaction = new Transaction().add(addTokenInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [
    admin,
  ]);

  console.log("\n--- Success! ---");
  console.log(`Transaction Signature: ${signature}`);
  console.log(`Token ${BLUE}${TOKEN_MINT.toBase58()}${RESET} at index ${GREEN}${TOKEN_TO_ADD.index}${RESET} has been added to the program.`);
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
