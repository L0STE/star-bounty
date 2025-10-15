import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { StarBounty } from "../target/types/star_bounty";
import { AddressLookupTableAccount, AddressLookupTableProgram, ComputeBudgetProgram, Connection, CreateLookupTableParams, ExtendLookupTableParams, Keypair, LAMPORTS_PER_SOL, PublicKey, sendAndConfirmTransaction, SystemProgram, SYSVAR_RENT_PUBKEY, Transaction, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountIdempotentInstruction, createMint, getAssociatedTokenAddressSync, Mint, TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { deriveCreatorAddress, deriveInvestorFeePositionOwnerAddress, deriveMetadataAccount, derivePositionNftMintAddress } from "./star";
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
  const user2Keypair = new Keypair();
  const user3Keypair = new Keypair();
  const user4Keypair = new Keypair();

  const admin = adminKeypair.publicKey;
  const user = userKeypair.publicKey;
  const user2 = user2Keypair.publicKey;
  const user3 = user3Keypair.publicKey;
  const user4 = user4Keypair.publicKey;

  const mintAKeypair = new Keypair();
  const mintA = mintAKeypair.publicKey;
  const mintB = new PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

  it("Setup", async () => {
    // Reset the admin key to an empty default account
    await surfnetSetAccount(connection, admin.toString(), 0, "", SystemProgram.programId.toString());

    // Airdrop to all addresses
    for (const address of [admin, user, user2, user3, user4]) {
      await surfnetAirdrop(connection, address.toString(), 1_000 * LAMPORTS_PER_SOL);
    }

    // Create the Mint
    await createMint(connection, adminKeypair, admin, null, 6, mintAKeypair, {skipPreflight: true})
  });

  let creator = deriveCreatorAddress(mintB);
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

  let senderTokens = getAssociatedTokenAddressSync(mintB, admin, true);
  let metadata = deriveMetadataAccount(creator, 0);
  let recipientTokens = getAssociatedTokenAddressSync(mintB, user, true);
  let streamflowTreasury = new PublicKey("5SEpbdjFK5FxwTvfsGMXVQTD2v4M2c5tyRTxhdsPkgDw");
  let streamflowTreasuryTokens = getAssociatedTokenAddressSync(mintB, streamflowTreasury, true);
  let withdrawor = new PublicKey("wdrwhnCv4pzW8beKsbPa4S2UDZrXenjg16KJdKSpb5u");
  let timelockProgram = new PublicKey("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m");
  let escrowTokens = PublicKey.findProgramAddressSync(
    [Buffer.from("strm"), metadata.toBuffer()],
    timelockProgram
  )[0];

  let metadata2 = deriveMetadataAccount(creator, 1);
  let recipientTokens2 = getAssociatedTokenAddressSync(mintB, user2, true);
  let escrowTokens2 = PublicKey.findProgramAddressSync(
    [Buffer.from("strm"), metadata2.toBuffer()],
    timelockProgram
  )[0];

  let metadata3 = deriveMetadataAccount(creator, 2);
  let recipientTokens3 = getAssociatedTokenAddressSync(mintB, user3, true);
  let escrowTokens3 = PublicKey.findProgramAddressSync(
    [Buffer.from("strm"), metadata3.toBuffer()],
    timelockProgram
  )[0];
  
  let metadata4 = deriveMetadataAccount(creator, 3);
  let recipientTokens4 = getAssociatedTokenAddressSync(mintB, user4, true);
  let escrowTokens4 = PublicKey.findProgramAddressSync(
    [Buffer.from("strm"), metadata4.toBuffer()],
    timelockProgram
  )[0];

  it("Create Stream", async () => {
    await surfnetTokenAirdrop(connection, admin.toString(), mintB.toString(), 1_000_000_000_000_000);

    const setupIx = [
      createAssociatedTokenAccountIdempotentInstruction(admin, recipientTokens, user, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, recipientTokens2, user2, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, recipientTokens3, user3, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, recipientTokens4, user4, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, streamflowTreasuryTokens, streamflowTreasury, mintB),
    ]

    const createStreamIx1 = await program.methods
      .createStream()
      .accountsStrict({
        creator,
        mint: mintB,
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

      const createStreamIx2 = await program.methods
      .createStream()
      .accountsStrict({
        creator,
        mint: mintB,
        sender: admin,
        senderTokens,
        recipient: user2,
        metadata:metadata2,
        escrowTokens: escrowTokens2,
        recipientTokens:recipientTokens2,
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

      const createStreamIx3 = await program.methods
      .createStream()
      .accountsStrict({
        creator,
        mint: mintB,
        sender: admin,
        senderTokens,
        recipient: user3,
        metadata:metadata3,
        escrowTokens: escrowTokens3,
        recipientTokens:recipientTokens3,
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

      const createStreamIx4 = await program.methods
      .createStream()
      .accountsStrict({
        creator,
        mint: mintB,
        sender: admin,
        senderTokens,
        recipient: user4,
        metadata:metadata4,
        escrowTokens: escrowTokens4,
        recipientTokens:recipientTokens4,
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

      const tx = new Transaction().add(...setupIx).add(createStreamIx1).add(createStreamIx2).add(createStreamIx3).add(createStreamIx4);
      await sendAndConfirmTransaction(connection, tx, [adminKeypair], {skipPreflight: true});
  });

  const investorFeePositionOwner = deriveInvestorFeePositionOwnerAddress(mintB);
  const investorPositionNftMint = derivePositionNftMintAddress(investorFeePositionOwner);
  const investorPositionNftAccount = derivePositionNftAccount(investorPositionNftMint);
  const investorPosition = derivePositionAccount(investorPositionNftMint);
  const investorTokenA = getAssociatedTokenAddressSync(mintA, investorFeePositionOwner, true);
  const investorTokenB = getAssociatedTokenAddressSync(mintB, investorFeePositionOwner, true);
  
  it("Deposit", async () => {
    await surfnetTokenAirdrop(connection, investorFeePositionOwner.toString(), mintA.toString(), 1_000_000_000_000_000);
    await surfnetTokenAirdrop(connection, investorFeePositionOwner.toString(), mintB.toString(), 1_000_000_000_000_000);

    const depositIx = await program.methods
      .deposit()
      .accountsStrict({
        investorFeePosOwner: investorFeePositionOwner,
        payer: admin,
        positionNftMint: investorPositionNftMint,
        positionNftAccount: investorPositionNftAccount,
        poolAuthority: POOL_AUTHORITY_ADDRESS,
        pool,
        position: investorPosition,
        mintA,
        mintB,
        tokenAVault,
        tokenBVault,
        payerTokenA: investorTokenA,
        payerTokenB: investorTokenB,
        tokenProgram: TOKEN_PROGRAM_ID,
        token2022Program: TOKEN_2022_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        eventAuthority: EVENT_AUTHORITY_ADDRESS,
        dammv2Program: DAMMV2_PROGRAM_ID,
      })
      .instruction()

      const tx = new Transaction().add(depositIx);
      await sendAndConfirmTransaction(connection, tx, [adminKeypair], {skipPreflight: true});
  });

  const adminTokenA = getAssociatedTokenAddressSync(mintA, admin, true);
  const adminTokenB = getAssociatedTokenAddressSync(mintB, admin, true);

  it("Swap", async () => {
    await surfnetTokenAirdrop(connection, admin.toString(), mintA.toString(), 1_000_000_000_000_000);

    const setupIx = [
      createAssociatedTokenAccountIdempotentInstruction(admin, adminTokenB, admin, mintB),
    ]

    const swapIx = await program.methods
      .swap(
        new BN(1_000_000)
      )
      .accountsStrict({
        payer: admin,
        poolAuthority: POOL_AUTHORITY_ADDRESS,
        pool,
        mintA,
        mintB,
        tokenAVault,
        tokenBVault,
        inputTokenAccount: adminTokenA,
        outputTokenAccount: adminTokenB,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: EVENT_AUTHORITY_ADDRESS,
        dammv2Program: DAMMV2_PROGRAM_ID,
      })
      .instruction()

      const tx = new Transaction().add(...setupIx).add(swapIx);
      await sendAndConfirmTransaction(connection, tx, [adminKeypair], {skipPreflight: true});
  });

  const creatorAddress = Keypair.generate().publicKey;
  const creatorTokenAccount = getAssociatedTokenAddressSync(mintB, creatorAddress, true);
  const userTokenAccount = getAssociatedTokenAddressSync(mintB, user, true);
  const user2TokenAccount = getAssociatedTokenAddressSync(mintB, user2, true);
  const user3TokenAccount = getAssociatedTokenAddressSync(mintB, user3, true);
  const user4TokenAccount = getAssociatedTokenAddressSync(mintB, user4, true);

  let lookupTable = PublicKey.default;

  it("Create a lookup table", async () => {
    let result = AddressLookupTableProgram.createLookupTable({
      authority: adminKeypair.publicKey,
      payer: adminKeypair.publicKey,
      recentSlot: await connection.getSlot(),
    } as CreateLookupTableParams);

    lookupTable = result[1];

    let tx = new Transaction();
    tx.instructions.push(result[0])
    await provider.sendAndConfirm(tx, [adminKeypair], { skipPreflight: true })
  })

  it("Extend lookup table", async () => {
    let tx = new Transaction();
    tx.instructions.push(AddressLookupTableProgram.extendLookupTable({
      lookupTable,
      authority: adminKeypair.publicKey,
      payer: adminKeypair.publicKey,
      addresses: [
        mintA,
        mintB,
        creator,
        investorFeePositionOwner,
        investorPositionNftMint,
        investorPositionNftAccount,
        pool,
        investorPosition,
        tokenAVault,
        tokenBVault,
        investorTokenA,
        investorTokenB,
        metadata,
        userTokenAccount,
        metadata2,
        user2TokenAccount,
        metadata3,
        user3TokenAccount,
        metadata4,
        user4TokenAccount,
      ],
    } as ExtendLookupTableParams))

    await provider.sendAndConfirm(tx, [adminKeypair])
    await new Promise(r => setTimeout(r, 2000));
  })

  it("Claim Fees", async () => {
    const setupIx = [
      createAssociatedTokenAccountIdempotentInstruction(admin, creatorTokenAccount, creatorAddress, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, userTokenAccount, user, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, user2TokenAccount, user2, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, user3TokenAccount, user3, mintB),
      createAssociatedTokenAccountIdempotentInstruction(admin, user4TokenAccount, user4, mintB),
    ]

    const claimFeesIx = await program.methods
      .claimFees()
      .accountsStrict({
        mintA,
        mintB,
        creator,
        creatorTokenAccount: creatorTokenAccount,
        investorFeePosOwner: investorFeePositionOwner,
        positionNftMint: investorPositionNftMint,
        positionNftAccount: investorPositionNftAccount,
        poolAuthority: POOL_AUTHORITY_ADDRESS,
        pool,
        position: investorPosition,
        tokenAVault,
        tokenBVault,
        tokenAAccount: investorTokenA,
        tokenBAccount: investorTokenB,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: EVENT_AUTHORITY_ADDRESS,
        dammv2Program: DAMMV2_PROGRAM_ID,
      })
      .remainingAccounts([
        {
          pubkey: metadata,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: userTokenAccount,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: metadata2,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: user2TokenAccount,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: metadata3,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: user3TokenAccount,
          isWritable: true,
          isSigner: false,
        },
        {
          pubkey: metadata4,
          isWritable: false,
          isSigner: false,
        },
        {
          pubkey: user4TokenAccount,
          isWritable: true,
          isSigner: false,
        }
      ])
      .instruction()

      const addressLookupTableAccounts: AddressLookupTableAccount[] = [];
    addressLookupTableAccounts.push((await connection.getAddressLookupTable(lookupTable)).value);

    const messageV0 = new TransactionMessage({
      payerKey: admin,
      recentBlockhash: (await connection.getLatestBlockhash()).blockhash,
      instructions: [
        ...setupIx,
        claimFeesIx,
      ],
    }).compileToV0Message(addressLookupTableAccounts)

    const tx = new VersionedTransaction(messageV0);
    tx.sign([adminKeypair]);

    await connection.sendTransaction(tx, {skipPreflight: true});
  });
});
