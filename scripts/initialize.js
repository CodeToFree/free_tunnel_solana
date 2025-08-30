import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as borsh from "borsh";
import path from "path";

const PROGRAM_KEYPAIR_PATH = path.join(
  "target",
  "deploy",
  "free_tunnel_solana-keypair.json"
);

const PROGRAM_ID = new PublicKey(
  "4y5qquCkpjqpMvkivnk7DYxekuX5ApKqcn4uFarjJVrj"
);

const RPC_URL = "http://127.0.0.1:8899";

const INSTRUCTION_SCHEMA = {
  struct: {
    is_mint_contract: 'u8',
    executors: { array: { type: { array: { type: 'u8', len: 20 } } } },
    threshold: 'u64',
    exe_index: 'u64'
  }
};

const GREEN="\x1b[32m";
const BLUE="\x1b[34m";
const RESET="\x1b[0m";


async function main() {
  // 1. Setup accounts
  console.log("\nConnecting to local validator...");
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("Setting up payer and program accounts...");
  const payer = Keypair.generate();

  console.log(`Airdropping 2 SOL to payer account: ${payer.publicKey.toBase58()}`);
  const airdropSignature = await connection.requestAirdrop(
    payer.publicKey,
    2 * 1_000_000_000 // 2 SOL
  );
  await connection.confirmTransaction(airdropSignature);
  console.log("Airdrop complete.");

  const isMintContract = true;
  const executors = [
    Buffer.alloc(20, 1), // Example executor 1
    Buffer.alloc(20, 2), // Example executor 2
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

  console.log(`PDA ${BLUE}[Basic Storage]${RESET}: ${basicStoragePda.toBase58()}`);
  console.log(`PDA ${BLUE}[Tokens/Proposers]${RESET}: ${tokensProposersPda.toBase58()}`);
  console.log(`PDA ${BLUE}[Executors (Index 0)]${RESET}: ${executorsAtIndexPda.toBase58()}`);


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
      // 0. account_payer
      { pubkey: payer.publicKey, isSigner: true, isWritable: true },
      // 1. account_admin
      { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      // 2. data_account_basic_storage
      { pubkey: basicStoragePda, isSigner: false, isWritable: true },
      // 3. data_account_tokens_proposers
      { pubkey: tokensProposersPda, isSigner: false, isWritable: true },
      // 4. data_account_executors_at_index
      { pubkey: executorsAtIndexPda, isSigner: false, isWritable: true },
      // 5. system_program
      { pubkey: new PublicKey("11111111111111111111111111111111"), isSigner: false, isWritable: false },
    ],
    data: instructionBuffer,
  });

  const transaction = new Transaction().add(initializeInstruction);

  console.log("Sending transaction...");
  const signature = await sendAndConfirmTransaction(connection, transaction, [
    payer,
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