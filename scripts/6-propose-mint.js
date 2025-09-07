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
const REQ_ID_PATH = path.join(TEMP_DIR, "reqid.bin");
const PROPOSAL_DETAILS_PATH = path.join(TEMP_DIR, "proposal_details.json");

// --- Instruction Data ---
const PROPOSAL_DETAILS = {
  version: 1,
  action: 1, // 1 for Mint
  tokenIndex: 56,
  amount: BigInt(123456),
  fromChain: 0xff,
  toChain: 0xff,
};

// Borsh schema for the ProposeMint instruction
const INSTRUCTION_SCHEMA = {
  struct: {
    req_id: { array: { type: 'u8', len: 32 } },
    recipient: { array: { type: 'u8', len: 32 } },
  }
};

const GREEN = "\x1b[32m";
const BLUE = "\x1b[34m";
const RESET = "\x1b[0m";
const YELLOW = "\x1b[33m";

/**
 * Loads the default Solana CLI keypair to act as the proposer/payer.
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
 * Creates, encodes, and saves the proposal data (ReqId and recipient).
 * @param {PublicKey} recipientPublicKey - The public key of the recipient.
 * @returns {Buffer} A 32-byte buffer representing the ReqId.
 */
function createAndSaveProposalData(recipientPublicKey) {
  const reqIdBuffer = Buffer.alloc(32);
  let offset = 0;

  // version: u8
  reqIdBuffer.writeUInt8(PROPOSAL_DETAILS.version, offset);
  offset += 1;

  // createdTime: u40 (5 bytes), Big-Endian
  const timestamp = BigInt(Math.floor(Date.now() / 1000));
  const timeBuffer = Buffer.alloc(8);
  timeBuffer.writeBigUInt64BE(timestamp);
  timeBuffer.copy(reqIdBuffer, offset, 3, 8); // Copy last 5 bytes
  offset += 5;

  // action: u8
  reqIdBuffer.writeUInt8(PROPOSAL_DETAILS.action, offset);
  offset += 1;

  // tokenIndex: u8
  reqIdBuffer.writeUInt8(PROPOSAL_DETAILS.tokenIndex, offset);
  offset += 1;

  // amount: u64, Big-Endian
  reqIdBuffer.writeBigUInt64BE(PROPOSAL_DETAILS.amount, offset);
  offset += 8;

  // from: u8
  reqIdBuffer.writeUInt8(PROPOSAL_DETAILS.fromChain, offset);
  offset += 1;

  // to: u8
  reqIdBuffer.writeUInt8(PROPOSAL_DETAILS.toChain, offset);
  offset += 1;

  // --- Save files ---
  if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
  }

  // Save binary ReqId
  fs.writeFileSync(REQ_ID_PATH, reqIdBuffer);
  console.log(`\n${GREEN}ReqId created and saved to:${RESET} ${BLUE}${REQ_ID_PATH}${RESET}`);
  console.log(`ReqId (hex): ${YELLOW}${reqIdBuffer.toString('hex')}${RESET}`);

  // Save full proposal details to JSON for later scripts
  const details = {
    reqIdHex: reqIdBuffer.toString('hex'),
    recipient: recipientPublicKey.toBase58()
  };
  fs.writeFileSync(PROPOSAL_DETAILS_PATH, JSON.stringify(details, null, 2));
  console.log(`Proposal details saved to: ${BLUE}${PROPOSAL_DETAILS_PATH}${RESET}`);

  return reqIdBuffer;
}


async function main() {
  // 1. Setup accounts and data
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Loading proposer/payer account from default Solana CLI path...");
  const proposer = loadAdminKeypair();
  console.log(`Using Proposer account: ${BLUE}${proposer.publicKey.toBase58()}${RESET}`);

  const recipient = Keypair.generate();
  console.log(`Generated recipient address: ${GREEN}${recipient.publicKey.toBase58()}${RESET}`);

  // Create and save both the binary ReqId and the JSON details file
  const reqId = createAndSaveProposalData(recipient.publicKey);

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
  const [proposedMintPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("mint"), reqId],
    PROGRAM_ID
  );

  console.log(`PDA ${BLUE}[Basic Storage]${RESET}: ${basicStoragePda.toBase58()}`);
  console.log(`PDA ${BLUE}[Tokens/Proposers]${RESET}: ${tokensProposersPda.toBase58()}`);
  console.log(`PDA ${BLUE}[Proposed Mint]${RESET}: ${proposedMintPda.toBase58()}`);

  // 3. Serialize instruction data
  const instructionDataPayload = {
    req_id: reqId,
    recipient: recipient.publicKey.toBuffer(),
  };

  const payloadBuffer = borsh.serialize(
    INSTRUCTION_SCHEMA,
    instructionDataPayload
  );

  // Prepend the instruction index (7 for ProposeMint)
  const instructionBuffer = Buffer.concat([
    Buffer.from([7]),
    payloadBuffer
  ]);

  // 4. Create and Send Transaction
  console.log("\nCreating ProposeMint instruction...");
  const proposeMintInstruction = new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      // 0. account_payer
      { pubkey: proposer.publicKey, isSigner: true, isWritable: true },
      // 1. account_proposer
      { pubkey: proposer.publicKey, isSigner: true, isWritable: false },
      // 2. data_account_basic_storage
      { pubkey: basicStoragePda, isSigner: false, isWritable: false },
      // 3. data_account_tokens_proposers
      { pubkey: tokensProposersPda, isSigner: false, isWritable: false },
      // 4. data_account_proposed_mint
      { pubkey: proposedMintPda, isSigner: false, isWritable: true },
      // 5. system_program
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: instructionBuffer,
  });

  const transaction = new Transaction().add(proposeMintInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [
    proposer,
  ]);

  console.log("\n--- Success! ---");
  console.log(`Transaction Signature: ${signature}`);
  console.log(`Successfully proposed to mint ${YELLOW}${PROPOSAL_DETAILS.amount}${RESET} of token index ${GREEN}${PROPOSAL_DETAILS.tokenIndex}${RESET} to recipient ${BLUE}${recipient.publicKey.toBase58()}${RESET}`);
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error("Error in main function:", err);
    process.exit(1);
  }
);
