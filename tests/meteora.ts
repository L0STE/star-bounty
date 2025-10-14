import { PublicKey } from "@solana/web3.js"

export const DAMMV2_PROGRAM_ID = new PublicKey("cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG");

export const EVENT_AUTHORITY_ADDRESS = PublicKey.findProgramAddressSync([Buffer.from("__event_authority")], DAMMV2_PROGRAM_ID)[0];
export const POOL_AUTHORITY_ADDRESS = new PublicKey("HLnpSz9h2S4hiLQ43rnSD9XkcUThA7B8hQMKmDaiTLcC");

/* Pool Helpers */
export const derivePoolAccount = (
    mintA: PublicKey,
    mintB: PublicKey
): PublicKey => {
    return PublicKey.findProgramAddressSync(
        [
            Buffer.from("cpool", "utf8"),
            ...[mintA.toBuffer(), mintB.toBuffer()].sort(Buffer.compare).reverse()
        ], DAMMV2_PROGRAM_ID
    )[0]
}

export const derivePositionNftAccount = (
    positionNftMint: PublicKey
): PublicKey => {
    return PublicKey.findProgramAddressSync(
        [Buffer.from("position_nft_account"), positionNftMint.toBuffer()], DAMMV2_PROGRAM_ID
    )[0]
}

export const derivePositionAccount = (
    positionNftMint: PublicKey
): PublicKey => {
    return PublicKey.findProgramAddressSync(
        [Buffer.from("position"), positionNftMint.toBuffer()], DAMMV2_PROGRAM_ID
    )[0]
}

export const deriveTokenVaultAccount = (
    mint: PublicKey,
    pool: PublicKey
): PublicKey => {
    return PublicKey.findProgramAddressSync(
        [Buffer.from("token_vault"), mint.toBuffer(), pool.toBuffer()], DAMMV2_PROGRAM_ID
    )[0]
}