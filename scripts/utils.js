import { Keypair } from "@solana/web3.js";
import fs from "fs";
import os from "os";
import path from "path";

/**
 * Loads the program keypair from the target/deploy directory.
 * @returns {{programKeypair: Keypair, programId: PublicKey}}
 */
export function loadProgramKeypair() {
  const PROGRAM_KEYPAIR_PATH = path.join(
    process.cwd(),
    "target",
    "deploy",
    "free_tunnel_solana-keypair.json"
  );

  if (!fs.existsSync(PROGRAM_KEYPAIR_PATH)) {
    throw new Error(
      `Program keypair not found at ${PROGRAM_KEYPAIR_PATH}. Please run 'cargo build-bpf' first.`
    );
  }

  const programSecretKey = JSON.parse(
    fs.readFileSync(PROGRAM_KEYPAIR_PATH, "utf-8")
  );
  const programKeypair = Keypair.fromSecretKey(new Uint8Array(programSecretKey));
  return {
    programKeypair,
    programId: programKeypair.publicKey
  };
}

/**
 * Loads a keypair from a specific file path.
 * @param {string} filePath - The path to the keypair file.
 * @returns {Keypair} The keypair.
 */
export function loadKeypairFromFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`Keypair file not found at: ${filePath}`);
  }
  const secretKey = JSON.parse(fs.readFileSync(filePath, 'utf-8'));
  return Keypair.fromSecretKey(new Uint8Array(secretKey));
}

/**
 * Loads the default Solana CLI keypair to act as the admin/payer.
 * @returns {Keypair} The keypair loaded from the default path.
 */
export function loadAdminKeypair() {
  const keypairPath = path.join(os.homedir(), '.config', 'solana', 'id.json');
  try {
      return loadKeypairFromFile(keypairPath);
  } catch (error) {
      throw new Error("Could not find Solana CLI keypair at default path. Please ensure it exists.");
  }
}

