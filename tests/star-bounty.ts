import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StarBounty } from "../target/types/star_bounty";
import { ComputeBudgetProgram, Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, sendAndConfirmTransaction, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountIdempotentInstruction, createMint, getAssociatedTokenAddressSync, Mint, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { deriveCreatorAddress, deriveMetadataAccount, derivePositionNftMintAddress } from "./star";
import { DAMMV2_PROGRAM_ID, derivePoolAccount, derivePositionAccount, derivePositionNftAccount, deriveTokenVaultAccount, EVENT_AUTHORITY_ADDRESS, POOL_AUTHORITY_ADDRESS } from "./meteora";

const adminSeed = [253,154,154,225,47,225,6,61,14,218,207,175,100,217,74,218,57,67,253,101,30,225,4,95,192,87,169,103,126,20,194,12,0,52,222,38,170,236,165,161,84,236,119,19,203,168,94,98,191,174,31,200,98,119,239,32,213,81,173,165,84,51,192,111];

// Surfnet Helpers
const surfnetAirdrop = async (connection: Connection, address: string, lamports: number) => {
  const call = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "surfnet_setAccount",
    "params": [address, { "lamports": lamports }]
  };
  
  await fetch(connection.rpcEndpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(call)
  });
};

const surfnetTokenAirdrop = async (connection: Connection, owner: string, mint: string, amount: number) => {
  const call = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "surfnet_setTokenAccount",
    "params": [owner, mint, {"amount": amount, "state": "initialized"}]
  };
  
  await fetch(connection.rpcEndpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(call)
  });
};

const surfnetTimeTravel = async (connection: Connection, timestamp: number) => {
  const call = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "surfnet_timeTravel",
    "params": [
      {
        "config": {
          "absoluteTimestamp": timestamp
        }
      }
    ]
  };

  await fetch(connection.rpcEndpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(call)
  });
};

const surfnetSetAccount = async (connection: Connection, account: string, lamports: number, data: string, owner: string) => {
  const call = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "surfnet_setAccount",
    "params": [account, {
      "lamports": lamports, 
      "data": data, 
      "owner": owner
    }]
  };

  await fetch(connection.rpcEndpoint, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(call)
  });
};

describe("star-bounty", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.starBounty as Program<StarBounty>;
  const provider = anchor.getProvider();
  const connection = provider.connection;

  const adminKeypair = Keypair.fromSecretKey(new Uint8Array(adminSeed));
  const userKeypair = new Keypair();

  const admin = adminKeypair.publicKey;
  const user = userKeypair.publicKey;

  const mintAKeypair = new Keypair();
  const mintA = mintAKeypair.publicKey;
  const mintB = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

  it("Setup", async () => {
    // Airdrop to all addresses
    for (const address of [admin, user]) {
      await surfnetAirdrop(connection, address.toString(), 1_000 * LAMPORTS_PER_SOL);
    }

    // Create the Mint
    await createMint(connection, adminKeypair, admin, null, 6, mintAKeypair)
  });

  let creator = deriveCreatorAddress(mintA);
  let positionNftMint = derivePositionNftMintAddress(creator);
  let positionNftAccount = derivePositionNftAccount(positionNftMint);
  let pool = derivePoolAccount(mintA, mintB);
  let position = derivePositionAccount(positionNftMint);
  let tokenAVault = deriveTokenVaultAccount(mintA, pool);
  let tokenBVault = deriveTokenVaultAccount(mintB, pool);

  let payerTokenA = getAssociatedTokenAddressSync(mintA, creator, true);
  let payerTokenB = getAssociatedTokenAddressSync(mintB, creator, true);

  it("Initialize Pool", async () => {
    await surfnetTokenAirdrop(connection, creator.toString(), mintA.toString(), 1_000_000_000_000_000);
    await surfnetTokenAirdrop(connection, creator.toString(), mintB.toString(), 1_000_000_000_000_000);

    const setComputeUnitLImitIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 300_000,
    });

    const initializePoolIx = await program.methods
      .initializePool()
      .accountsStrict({
        payer: admin,
        creator,
        positionNftMint,
        positionNftAccount,
        poolAuthority: POOL_AUTHORITY_ADDRESS,
        pool,
        position,
        mintA,
        mintB,
        tokenAVault,
        tokenBVault,
        payerTokenA,
        payerTokenB,
        tokenProgram: TOKEN_PROGRAM_ID,
        token2022Program: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        eventAuthority: EVENT_AUTHORITY_ADDRESS,
        dammv2Program: DAMMV2_PROGRAM_ID,
      })
      .instruction()

      const tx = new Transaction().add(setComputeUnitLImitIx).add(initializePoolIx);
      await sendAndConfirmTransaction(connection, tx, [adminKeypair]);
  });

  let senderTokens = getAssociatedTokenAddressSync(mintA, admin, true);
  let metadata = deriveMetadataAccount(creator, 0);
  let recipientTokens = getAssociatedTokenAddressSync(mintA, user, true);
  let streamflowTreasury = new PublicKey("5SEpbdjFK5FxwTvfsGMXVQTD2v4M2c5tyRTxhdsPkgDw");
  let streamflowTreasuryTokens = getAssociatedTokenAddressSync(mintA, streamflowTreasury, true);
  let withdrawor = new PublicKey("wdrwhnCv4pzW8beKsbPa4S2UDZrXenjg16KJdKSpb5u");
  let timelockProgram = new PublicKey("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m");
  let escrowTokens = PublicKey.findProgramAddressSync(
    [Buffer.from("strm"), metadata.toBuffer()],
    timelockProgram
  )[0];

  it("Create Stream", async () => {
    await surfnetTokenAirdrop(connection, admin.toString(), mintA.toString(), 1_000_000_000_000_000);

    const setupIx = [
      createAssociatedTokenAccountIdempotentInstruction(admin, recipientTokens, user, mintA),
      createAssociatedTokenAccountIdempotentInstruction(admin, streamflowTreasuryTokens, streamflowTreasury, mintA),
    ]

    const createStreamIx = await program.methods
      .createStream()
      .accountsStrict({
        creator,
        mint: mintA,
        sender: admin,
        senderTokens,
        recipient: user,
        metadata,
        escrowTokens,
        recipientTokens,
        streamflowTreasury,
        streamflowTreasuryTokens,
        withdrawor,
        timelockProgram,
        rent: SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .instruction()

      const tx = new Transaction().add(...setupIx).add(createStreamIx);
      await sendAndConfirmTransaction(connection, tx, [adminKeypair], {skipPreflight: true});
  });
});
