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
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID
} from "@solana/spl-token";
import * as borsh from "borsh";
import fs from "fs";
import path from "path";
import { loadProgramKeypair, loadKeypairFromFile, loadAdminKeypair } from "./utils.js";

// --- Configuration ---
const { programId: PROGRAM_ID } = loadProgramKeypair();

const RPC_URL = "http://127.0.0.1:8899";
const TEMP_DIR = path.join("scripts", "temp");
const TOKEN_DETAILS_PATH = path.join(TEMP_DIR, "token_details.json");
const PROPOSAL_DETAILS_PATH = path.join(TEMP_DIR, "proposal_details.json");
const PDAS_FILE_PATH = path.join(TEMP_DIR, "program_pdas.json");
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
  const admin = loadAdminKeypair();
  console.log(`Using Admin/Payer account: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);

  console.log("Loading details from temp files...");
  const tokenDetails = loadJsonFile(TOKEN_DETAILS_PATH);
  const proposalDetails = loadJsonFile(PROPOSAL_DETAILS_PATH);
  const programPdas = loadJsonFile(PDAS_FILE_PATH);
  const reqId = fs.readFileSync(REQ_ID_PATH);

  const tokenMint = new PublicKey(tokenDetails.tokenMint);
  const multisigAddress = new PublicKey(tokenDetails.multisigAddress);
  const recipient = new PublicKey(proposalDetails.recipient);
  const contractSignerPda = new PublicKey(programPdas.contractSigner);

  // 2. Prepare recipient's token account
  console.log(`\nPreparing token account for recipient: ${BLUE}${recipient.toBase58()}${RESET}`);
  const mintAccountInfo = await connection.getAccountInfo(tokenMint);
  if (!mintAccountInfo) {
    throw new Error("Token mint account not found on-chain.");
  }
  const tokenProgramId = mintAccountInfo.owner.equals(TOKEN_2022_PROGRAM_ID)
    ? TOKEN_2022_PROGRAM_ID
    : TOKEN_PROGRAM_ID;

  const recipientTokenAccount = await getAssociatedTokenAddress(
    tokenMint,
    recipient,
    false,
    tokenProgramId,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );
  console.log(`Recipient's Associated Token Account: ${GREEN}${recipientTokenAccount.toBase58()}${RESET}`);

  const transaction = new Transaction();

  const accountInfo = await connection.getAccountInfo(recipientTokenAccount);
  if (accountInfo === null) {
    console.log("Recipient token account does not exist. Adding instruction to create it...");
    transaction.add(
      createAssociatedTokenAccountInstruction(
        admin.publicKey, // Payer
        recipientTokenAccount,
        recipient,
        tokenMint,
        tokenProgramId,
        ASSOCIATED_TOKEN_PROGRAM_ID
      )
    );
  } else {
    console.log("Recipient token account already exists.");
  }

  // 3. Calculate remaining PDA addresses
  console.log("\nCalculating PDA addresses...");
  const [proposedMintPda] = PublicKey.findProgramAddressSync([Buffer.from("mint"), reqId], PROGRAM_ID);

  const exeIndex = BigInt(0);
  const exeIndexBuffer = Buffer.alloc(8);
  exeIndexBuffer.writeBigUInt64LE(exeIndex);
  const [currentExecutorsPda] = PublicKey.findProgramAddressSync([Buffer.from("executors"), exeIndexBuffer], PROGRAM_ID);

  console.log(`PDA ${BLUE}[Proposed Mint]${RESET}: ${proposedMintPda.toBase58()}`);
  console.log(`PDA ${BLUE}[Current Executors]${RESET}: ${currentExecutorsPda.toBase58()}`);

  // 4. Serialize instruction data
  const instructionDataPayload = {
    req_id: reqId,
    signatures: [],
    executors: [],
    exe_index: exeIndex,
  };

  const payloadBuffer = borsh.serialize(INSTRUCTION_SCHEMA, instructionDataPayload);
  const instructionBuffer = Buffer.concat([Buffer.from([8]), payloadBuffer]); // 8 for ExecuteMint

  // 5. Create and Send Transaction
  console.log("\nCreating ExecuteMint instruction...");
  const executeMintInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      // 0. token_program
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
      // 1. account_contract_signer (The PDA is the sole signer for the CPI)
      { pubkey: contractSignerPda, isSigner: false, isWritable: false },
      // 2. token_account_recipient
      { pubkey: recipientTokenAccount, isSigner: false, isWritable: true },
      // 3. data_account_basic_storage
      { pubkey: new PublicKey(programPdas.basicStorage), isSigner: false, isWritable: false },
      // 4. data_account_proposed_mint
      { pubkey: proposedMintPda, isSigner: false, isWritable: true },
      // 5. data_account_executors
      { pubkey: currentExecutorsPda, isSigner: false, isWritable: false },
      // 6. token_mint
      { pubkey: tokenMint, isSigner: false, isWritable: true },
      // 7. account_multisig_owner
      { pubkey: multisigAddress, isSigner: false, isWritable: false },
    ],
    data: instructionBuffer,
  });

  transaction.add(executeMintInstruction);

  console.log("Sending transaction...");
  // Only the admin/payer needs to sign this transaction
  const signature = await sendAndConfirmTransaction(connection, transaction, [admin]);

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
