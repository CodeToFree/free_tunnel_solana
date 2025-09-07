import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  getAssociatedTokenAddress,
  createAssociatedTokenAccountInstruction,
  TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import * as borsh from "borsh";
import fs from "fs";
import os from "os";
import path from "path";

// --- Configuration ---
const PROGRAM_ID = new PublicKey(
  "4y5qquCkpjqpMvkivnk7DYxekuX5ApKqcn4uFarjJVrj"
);
const RPC_URL = "http://127.0.0.1:8899";
const TEMP_DIR = path.join("scripts", "temp");
const SIGNER1_KEYPAIR_PATH = path.join("keys", "signer1.json");
const TOKEN_DETAILS_PATH = path.join(TEMP_DIR, "token_details.json");
const PROPOSAL_DETAILS_PATH = path.join(TEMP_DIR, "proposal_details.json");
const REQ_ID_PATH = path.join(TEMP_DIR, "reqid.bin");

// Borsh schema for the ExecuteMint instruction
const INSTRUCTION_SCHEMA = {
  struct: {
    req_id: { array: { type: 'u8', len: 32 } },
    signatures: { array: { type: { array: { type: 'u8', len: 64 } } } },
    executors: { array: { type: { array: { type: 'u8', len: 20 } } } },
    exe_index: 'u64'
  }
};

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

/**
 * Loads a keypair from a specific file path.
 * @param {string} filePath - The path to the keypair file.
 * @returns {Keypair} The keypair.
 */
function loadKeypairFromFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Keypair file not found at: ${filePath}`);
  }
  const secretKey = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  return Keypair.fromSecretKey(new Uint8Array(secretKey));
}

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
  // 1. Setup and load all necessary data
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading admin/payer account...");
  const admin = loadKeypairFromFile(path.join(os.homedir(), '.config', 'solana', 'id.json'));
  const signer1 = loadKeypairFromFile(SIGNER1_KEYPAIR_PATH);
  console.log(`Using Admin/Payer account: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);
  console.log(`Using Multisig Signer #1: ${BLUE}${signer1.publicKey.toBase58()}${RESET}`);
  console.log(`Using Admin account: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);

  console.log("Loading details from temp files...");
  const tokenDetails = loadJsonFile(TOKEN_DETAILS_PATH);
  const proposalDetails = loadJsonFile(PROPOSAL_DETAILS_PATH);
  const reqId = fs.readFileSync(REQ_ID_PATH);

  const tokenMint = new PublicKey(tokenDetails.tokenMint);
  const multisigAddress = new PublicKey(tokenDetails.multisigAddress);
  const multisigSigners = tokenDetails.multisigSigners.map(pk => new PublicKey(pk));
  const recipient = new PublicKey(proposalDetails.recipient);

  // 2. Prepare recipient's token account
  console.log(`\nPreparing token account for recipient: ${BLUE}${recipient.toBase58()}${RESET}`);
  const recipientTokenAccount = await getAssociatedTokenAddress(tokenMint, recipient);
  console.log(`Recipient's Associated Token Account: ${GREEN}${recipientTokenAccount.toBase58()}${RESET}`);

  const transaction = new Transaction();

  // Check if the recipient's token account exists. If not, add an instruction to create it.
  const accountInfo = await connection.getAccountInfo(recipientTokenAccount);
  if (accountInfo === null) {
    console.log("Recipient token account does not exist. Adding instruction to create it...");
    transaction.add(
      createAssociatedTokenAccountInstruction(
        admin.publicKey, // Payer
        recipientTokenAccount,
        recipient,
        tokenMint
      )
    );
  } else {
    console.log("Recipient token account already exists.");
  }

  // 3. Calculate PDA addresses
  console.log("\nCalculating PDA addresses...");
  const [basicStoragePda] = PublicKey.findProgramAddressSync([Buffer.from("basic-storage")], PROGRAM_ID);
  const [tokensProposersPda] = PublicKey.findProgramAddressSync([Buffer.from("tokens-proposers")], PROGRAM_ID);
  const [proposedMintPda] = PublicKey.findProgramAddressSync([Buffer.from("mint"), reqId], PROGRAM_ID);

  const exeIndex = BigInt(0);
  const exeIndexBuffer = Buffer.alloc(8);
  exeIndexBuffer.writeBigUInt64LE(exeIndex);
  const [currentExecutorsPda] = PublicKey.findProgramAddressSync([Buffer.from("executors"), exeIndexBuffer], PROGRAM_ID);

  // For `next_executors`, we calculate the PDA for index + 1. The program should handle cases where it doesn't exist.
  const nextExeIndexBuffer = Buffer.alloc(8);
  nextExeIndexBuffer.writeBigUInt64LE(exeIndex + BigInt(1));
  const [nextExecutorsPda] = PublicKey.findProgramAddressSync([Buffer.from("executors"), nextExeIndexBuffer], PROGRAM_ID);

  console.log(`PDA ${BLUE}[Proposed Mint]${RESET}: ${proposedMintPda.toBase58()}`);
  console.log(`PDA ${BLUE}[Current Executors]${RESET}: ${currentExecutorsPda.toBase58()}`);

  // 4. Serialize instruction data
  // As decided, we pass empty arrays for signatures and executors to bypass the check on-chain.
  const instructionDataPayload = {
    req_id: reqId,
    signatures: [],
    executors: [],
    exe_index: exeIndex,
  };

  const payloadBuffer = borsh.serialize(INSTRUCTION_SCHEMA, instructionDataPayload);
  const instructionBuffer = Buffer.concat([Buffer.from([9]), payloadBuffer]); // 9 for ExecuteMint

  // 5. Create and Send Transaction
  console.log("\nCreating ExecuteMint instruction...");
  const executeMintInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      // 0. system_account_token_program
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      // 1. data_account_basic_storage
      { pubkey: basicStoragePda, isSigner: false, isWritable: false },
      // 2. data_account_tokens_proposers
      { pubkey: tokensProposersPda, isSigner: false, isWritable: false },
      // 3. data_account_proposed_mint
      { pubkey: proposedMintPda, isSigner: false, isWritable: true },
      // 4. data_account_current_executors
      { pubkey: currentExecutorsPda, isSigner: false, isWritable: false },
      // 5. data_account_next_executors
      { pubkey: nextExecutorsPda, isSigner: false, isWritable: false },
      // 6. token_account_recipient
      { pubkey: recipientTokenAccount, isSigner: false, isWritable: true },
      // 7. account_token_mint
      { pubkey: tokenMint, isSigner: false, isWritable: true },
      // 8. account_multisig_owner
      { pubkey: multisigAddress, isSigner: false, isWritable: false },
      // 9..n account_multisig_wallets
      { pubkey: signer1.publicKey, isSigner: true, isWritable: false },
    ],
    data: instructionBuffer,
  });

  transaction.add(executeMintInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [admin, signer1]);

  console.log("\n--- Success! ---");
  console.log(`Transaction Signature: ${signature}`);
  console.log(`Successfully executed the mint proposal for recipient ${BLUE}${recipient.toBase58()}${RESET}`);
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error("Error in main function:", err);
    process.exit(1);
  }
);

