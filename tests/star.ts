import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

export const STAR_BOUNTY_PROGRAM_ID = new PublicKey("Ek8UkGyAXwg9qBPn82BEdNHjDxFHDjXhUQmRrTzmczxa");

export function deriveCreatorAddress(mint: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("creator", "utf8"), mint.toBuffer()],
    STAR_BOUNTY_PROGRAM_ID
  )[0];
}

export function derivePositionNftMintAddress(
    creator: PublicKey
  ): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("position_mint"), creator.toBuffer()],
      STAR_BOUNTY_PROGRAM_ID
    )[0];
  }

export function deriveMetadataAccount(creator: PublicKey, stream: number): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), creator.toBuffer(), Buffer.from([stream])],
    STAR_BOUNTY_PROGRAM_ID
  )[0];
}