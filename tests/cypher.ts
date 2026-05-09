import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { CyperV0 } from "../target/types/cyper_v0";
import { 
  PublicKey, 
  Keypair, 
  SystemProgram, 
  LAMPORTS_PER_SOL 
} from "@solana/web3.js";
import { 
  TOKEN_PROGRAM_ID, 
  createMint, 
  getAssociatedTokenAddressSync, 
  mintTo,
  ASSOCIATED_TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import { expect } from "chai";

describe("cyper_v0", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.CyperV0 as Program<CyperV0>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  let mint: PublicKey;
  let treasuryAta: PublicKey;
  let protocolPda: PublicKey;
  let marketPda: PublicKey;
  let marketVault: PublicKey;

  it("Is initialized!", async () => {
    console.log("--- STARTING TEST: Is initialized! ---");
    // 1. Create Mint
    mint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );
    console.log("Mint created:", mint.toBase58());

    // 2. Derive Protocol PDA
    [protocolPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("protocol")],
      program.programId
    );
    console.log("Protocol PDA:", protocolPda.toBase58());

    // 3. Derive Treasury ATA (don't create it, let program do it)
    treasuryAta = getAssociatedTokenAddressSync(
      mint,
      admin.publicKey
    );
    console.log("Treasury ATA:", treasuryAta.toBase58());

    // 4. Initialize
    try {
        const tx = await program.methods
          .initialize(50, new anchor.BN(0)) // 0.5% fee, 0 bond
          .accounts({
            authority: admin.publicKey,
            mint: mint,
            treasury: treasuryAta,
            market: protocolPda,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          })
          .rpc();

        console.log("Initialize TX:", tx);
    } catch (e) {
        console.error("Initialize FAILED:", e);
        throw e;
    }

    const protocolAccount = await program.account.cyperMarket.fetch(protocolPda);
    console.log("Protocol Account Data:", protocolAccount);
    expect(protocolAccount.authority.toBase58()).to.equal(admin.publicKey.toBase58());
    expect(protocolAccount.defaultProtocolFeeBps).to.equal(50);
  });

  it("Creates a Yes/No Market", async () => {
    const marketIndex = new anchor.BN(0);
    [marketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), marketIndex.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    // Derive Market Vault ATA
    marketVault = getAssociatedTokenAddressSync(
      mint,
      marketPda,
      true
    );

    const adminAta = getAssociatedTokenAddressSync(
        mint,
        admin.publicKey
    );

    const deadline = new anchor.BN(Math.floor(Date.now() / 1000) + 3600); // 1 hour from now

    const tx = await program.methods
      .createMarket(
        "Will SOL hit $500 in 2025?",
        new anchor.BN(0), // fixed_price (not used for YesNo)
        { yesNo: {} },   // market_type
        { crypto: {} },  // category
        50,              // lp_fee_bps
        deadline,
        null,            // market_group
        { yesNo: { yesPool: new anchor.BN(0), noPool: new anchor.BN(0) } } // market_data
      )
      .accounts({
        marketAuthority: admin.publicKey,
        cyperMarket: protocolPda,
        market: marketPda,
        mint: mint,
        marketVaultAta: marketVault,
        marketAuthorityAta: adminAta,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Create Market TX:", tx);

    const marketAccount = await program.account.market.fetch(marketPda);
    expect(marketAccount.question).to.equal("Will SOL hit $500 in 2025?");
  });

  it("Places a Bet", async () => {
    const amount = new anchor.BN(10 * 10**6); // 10 tokens

    const adminAta = getAssociatedTokenAddressSync(
        mint,
        admin.publicKey
    );

    // Mint some tokens to admin first (ATA should already exist from Initialize)
    await mintTo(
        provider.connection,
        admin,
        mint,
        adminAta,
        admin,
        100 * 10**6
    );

    const betIndex = new anchor.BN(0);
    const [betPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), marketPda.toBuffer(), betIndex.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const tx = await program.methods
      .placeBet(amount, { yesNo: { direction: true } }) // Betting YES
      .accounts({
        better: admin.publicKey,
        mint: mint,
        market: marketPda,
        bet: betPda,
        betterVault: adminAta,
        marketVault: marketVault,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Place Bet TX:", tx);

    const betAccount = await program.account.bet.fetch(betPda);
    expect(betAccount.amount.toNumber()).to.equal(amount.toNumber());
  });

  it("Creates an Accuracy Market", async () => {
    const marketIndex = new anchor.BN(1);
    const [accuracyMarketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), marketIndex.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const accuracyMarketVault = getAssociatedTokenAddressSync(
      mint,
      accuracyMarketPda,
      true
    );

    const adminAta = getAssociatedTokenAddressSync(
        mint,
        admin.publicKey
    );

    const deadline = new anchor.BN(Math.floor(Date.now() / 1000) + 3600); // 1 hour from now
    const fixedPrice = new anchor.BN(5 * 10**6); // 5 tokens

    const tx = await program.methods
      .createMarket(
        "What will be the price of SOL on Dec 31?",
        fixedPrice, 
        { accuracy: { fixedPrice: fixedPrice } }, // market_type
        { crypto: {} },                           // category
        null,                                     // lp_fee_bps must be null/0 for accuracy
        deadline,
        null,                                     // market_group
        { accuracy: { totalPool: new anchor.BN(0) } } // market_data
      )
      .accounts({
        marketAuthority: admin.publicKey,
        cyperMarket: protocolPda,
        market: accuracyMarketPda,
        mint: mint,
        marketVaultAta: accuracyMarketVault,
        marketAuthorityAta: adminAta,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Create Accuracy Market TX:", tx);

    const marketAccount = await program.account.market.fetch(accuracyMarketPda);
    expect(marketAccount.question).to.equal("What will be the price of SOL on Dec 31?");
  });

  it("Places an Accuracy Bet", async () => {
    const marketIndex = new anchor.BN(1);
    const [accuracyMarketPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("market"), marketIndex.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const accuracyMarketVault = getAssociatedTokenAddressSync(
      mint,
      accuracyMarketPda,
      true
    );

    const amount = new anchor.BN(5 * 10**6); // Must match fixedPrice
    const adminAta = getAssociatedTokenAddressSync(
        mint,
        admin.publicKey
    );

    const betIndex = new anchor.BN(0);
    const [betPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("bet"), accuracyMarketPda.toBuffer(), betIndex.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const tx = await program.methods
      .placeBet(amount, { accuracy: { predictedValue: new anchor.BN(500) } })
      .accounts({
        better: admin.publicKey,
        mint: mint,
        market: accuracyMarketPda,
        bet: betPda,
        betterVault: adminAta,
        marketVault: accuracyMarketVault,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();

    console.log("Place Accuracy Bet TX:", tx);

    const betAccount = await program.account.bet.fetch(betPda);
    expect(betAccount.amount.toNumber()).to.equal(amount.toNumber());
  });
});
