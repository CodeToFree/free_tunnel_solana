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
import os from "os";
import path from "path";

// --- Configuration ---
const PROGRAM_ID = new PublicKey(
  "4y5qquCkpjqpMvkivnk7DYxekuX5ApKqcn4uFarjJVrj"
);

const RPC_URL = "http://127.0.0.1:8899";
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
 * Saves the calculated PDA addresses to a JSON file.
 * @param {object} pdas - The pda addresses to save.
 */
function savePdasToFile(pdas) {
  if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
  }
  fs.writeFileSync(PDAS_FILE_PATH, JSON.stringify(pdas, null, 2));
  console.log(`\n${GREEN}Successfully saved PDA addresses to ${BLUE}${PDAS_FILE_PATH}${RESET}`);
}

async function main() {
  // 1. Setup accounts
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading admin/payer account from default Solana CLI path...");
  const admin = loadAdminKeypair();
  console.log(`Using Admin account: ${BLUE}${admin.publicKey.toBase58()}${RESET}`);

  // Airdrop if needed (useful for first-time runs on a fresh validator)
  // await connection.requestAirdrop(admin.publicKey, 2 * 1_000_000_000);

  const isMintContract = true;
  const executors = [
    Buffer.alloc(20, 1),
    Buffer.alloc(20, 2),
  ];
  const threshold = BigInt(2);
  const exeIndex = BigInt(0);


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
    tokensProposers: tokensProposersPda.toBase58(),
    executorsAtIndex0: executorsAtIndexPda.toBase58(),
    contractSigner: contractSignerPda.toBase58(),
  };

  console.log(`PDA ${BLUE}[Basic Storage]${RESET}: ${pdas.basicStorage}`);
  console.log(`PDA ${BLUE}[Tokens/Proposers]${RESET}: ${pdas.tokensProposers}`);
  console.log(`PDA ${BLUE}[Executors (Index 0)]${RESET}: ${pdas.executorsAtIndex0}`);
  console.log(`PDA ${BLUE}[Contract Signer]${RESET}: ${pdas.contractSigner}`);

  // Save the PDAs for other scripts to use
  savePdasToFile(pdas);

  // 3. Serialize instruction data
  const instructionDataPayload = {
    is_mint_contract: isMintContract ? 1 : 0,
    executors: executors,
    threshold: threshold,
    exe_index: exeIndex,
  };

  const payloadBuffer = borsh.serialize(
    INSTRUCTION_SCHEMA,
    instructionDataPayload
  );

  const instructionBuffer = Buffer.concat([
    Buffer.from([0]),
    payloadBuffer
  ]);

  // 4. Create and Send Transaction
  console.log("\nCreating Initialize instruction...");
  const initializeInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: admin.publicKey, isSigner: true, isWritable: true },
      { pubkey: admin.publicKey, isSigner: true, isWritable: false },
      { pubkey: basicStoragePda, isSigner: false, isWritable: true },
      { pubkey: tokensProposersPda, isSigner: false, isWritable: true },
      { pubkey: executorsAtIndexPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: instructionBuffer,
  });

  const transaction = new Transaction().add(initializeInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [
    admin,
  ]);

  console.log("\n--- Success! ---");
  console.log(`Transaction Signature: ${signature}`);
  console.log("Your program has been initialized.");
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error(err);
    process.exit(1);
  }
);
