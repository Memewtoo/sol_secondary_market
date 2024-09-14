import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SecMarket } from "../target/types/sec_market";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import { expect } from "chai";

describe("sec_market", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SecMarket as Program<SecMarket>;

  let vaultTokenMint: PublicKey;
  let usdcMint: PublicKey;
  let creatorTokenAccount: PublicKey;
  let buyerTokenAccount: PublicKey;
  let creatorUsdcAccount: PublicKey;
  let buyerUsdcAccount: PublicKey;
  let escrowTokenAccount: PublicKey;
  let orderPDA: PublicKey;
  let orderBump: number;

  const creator = Keypair.generate();
  const buyer = Keypair.generate();
  const seed = new anchor.BN(Math.floor(Math.random() * 1000000));
  const price = new anchor.BN(1); // 1 USDC (assuming 6 decimals)
  const amount = new anchor.BN(10);
  const expiration = new anchor.BN(1); // 1 day

  before(async () => {
    // Airdrop SOL to creator and buyer
    const airdropCreator = await provider.connection.requestAirdrop(creator.publicKey, 10 * LAMPORTS_PER_SOL);
    
    const latestBlockHash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropCreator,
    });

    const airdropBuyer = await provider.connection.requestAirdrop(buyer.publicKey, 10 * LAMPORTS_PER_SOL);
    
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: airdropBuyer,
    });

    // Create USDC mint
    usdcMint = await createMint(provider.connection, creator, creator.publicKey, null, 6);

    // Create USDC accounts for creator and buyer
    creatorUsdcAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      creator,
      usdcMint,
      creator.publicKey
    ).then(account => account.address);

    buyerUsdcAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      buyer,
      usdcMint,
      buyer.publicKey
    ).then(account => account.address);

    // Mint USDC to creator and buyer
    await mintTo(provider.connection, creator, usdcMint, creatorUsdcAccount, creator, 1000_000_000); // 1000 USDC
    await mintTo(provider.connection, creator, usdcMint, buyerUsdcAccount, creator, 1000_000_000); // 1000 USDC

    // Create vault token mint and accounts
    vaultTokenMint = await createMint(provider.connection, creator, creator.publicKey, null, 9);
    creatorTokenAccount = await createAccount(provider.connection, creator, vaultTokenMint, creator.publicKey);
    buyerTokenAccount = await createAccount(provider.connection, buyer, vaultTokenMint, buyer.publicKey);

    // Mint vault tokens to creator
    await mintTo(provider.connection, creator, vaultTokenMint, creatorTokenAccount, creator, 100 * LAMPORTS_PER_SOL);

    // Calculate PDA for order
    [orderPDA, orderBump] = await PublicKey.findProgramAddressSync(
      [Buffer.from("order"), creator.publicKey.toBuffer(), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    // Calculate PDA for escrow token account
    escrowTokenAccount = await anchor.utils.token.associatedAddress({
      mint: vaultTokenMint,
      owner: orderPDA
    });

    // Log balances for debugging
    const creatorBalance = await provider.connection.getBalance(creator.publicKey);
    const buyerBalance = await provider.connection.getBalance(buyer.publicKey);
    const creatorUsdcBalance = await provider.connection.getTokenAccountBalance(creatorUsdcAccount);
    const buyerUsdcBalance = await provider.connection.getTokenAccountBalance(buyerUsdcAccount);

    console.log(`Creator SOL balance: ${creatorBalance / LAMPORTS_PER_SOL} SOL`);
    console.log(`Buyer SOL balance: ${buyerBalance / LAMPORTS_PER_SOL} SOL`);
    console.log(`Creator USDC balance: ${creatorUsdcBalance.value.uiAmount} USDC`);
    console.log(`Buyer USDC balance: ${buyerUsdcBalance.value.uiAmount} USDC`);
  });

  it("Creates an order", async () => {
    const create_order_tx = await program.methods
      .createOrder(seed, price, amount, expiration)
      .accounts({
        creator: creator.publicKey,
        vaultTokenMint,
        priceTokenMint: usdcMint,
        creatorTokenAccount,
      })
      .signers([creator])
      .rpc();

    const latestBlockHash = await provider.connection.getLatestBlockhash();
  
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: create_order_tx,
    });

    const orderAccount = await program.account.order.fetch(orderPDA);
    expect(orderAccount.creator.toBase58()).to.equal(creator.publicKey.toBase58());
    expect(orderAccount.amount.toNumber()).to.equal(amount.toNumber());
    expect(orderAccount.price.toNumber()).to.equal(price.toNumber());
  });

  it("Modifies an order", async () => {
    console.log("Modifying order with PDA:", orderPDA.toBase58());
    
    // Check if the order account exists before trying to modify it
    try {
      const orderAccount = await program.account.order.fetch(orderPDA);
      console.log("Order account exists before modification:", orderAccount);
    } catch (error) {
      console.error("Error fetching order account before modification:", error);
      throw error;
    }

    const newAmount = new anchor.BN(15);
    const newPrice = new anchor.BN(2); // 2 USDC
    const newDuration = new anchor.BN(2); // 2 days
  
    const modify_order_tx = await program.methods
      .modifyOrder(newAmount, newPrice, newDuration)
      .accountsPartial({
        creator: creator.publicKey,
        order: orderPDA,
        escrowTokenAccount,
        creatorTokenAccount,
      })
      .signers([creator])
      .rpc();
    
    const latestBlockHash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: modify_order_tx,
    });

    console.log("Order modified successfully");

    const modifiedOrder = await program.account.order.fetch(orderPDA);
    expect(modifiedOrder.amount.toNumber()).to.equal(newAmount.toNumber());
    expect(modifiedOrder.price.toNumber()).to.equal(newPrice.toNumber());

    console.log("Modified Order: ", orderPDA);
    console.log("Amount: ", modifiedOrder.amount.toNumber());
    console.log("Price: ", modifiedOrder.price.toNumber());
  });

  it("Buys tokens", async () => {
    const buyAmount = new anchor.BN(5); // Purchase 5 Tokens

    const buy_token_tx = await program.methods.buyTokens(buyAmount)
      .accountsPartial({
        buyer: buyer.publicKey,
        creator: creator.publicKey,
        priceTokenMint: usdcMint,
        order: orderPDA,
        escrowTokenAccount,
        vaultTokenMint,
        buyerPriceTokenAccount: buyerUsdcAccount,
        creatorPriceTokenAccount: creatorUsdcAccount,
      })
      .signers([buyer, creator])
      .rpc();

    const latestBlockHash = await provider.connection.getLatestBlockhash();

    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: buy_token_tx,
    });

    const updatedOrder = await program.account.order.fetch(orderPDA);
    expect(updatedOrder.remainingAmount.toNumber()).to.equal(10);

    // Check USDC balances after purchase
    const buyerUsdcBalance = await provider.connection.getTokenAccountBalance(buyerUsdcAccount);
    const creatorUsdcBalance = await provider.connection.getTokenAccountBalance(creatorUsdcAccount);
    console.log(`Buyer USDC balance after purchase: ${buyerUsdcBalance.value.uiAmount} USDC`);
    console.log(`Creator USDC balance after purchase: ${creatorUsdcBalance.value.uiAmount} USDC`);
  });

  it("Fails to cancel a partially filled order", async () => {
    try {
      const cancel_order_tx = await program.methods.cancelOrder()
        .accountsPartial({
          creator: creator.publicKey,
          escrowTokenAccount,
          creatorTokenAccount,
          order: orderPDA,
        })
        .signers([creator])
        .rpc();
      
      const latestBlockHash = await provider.connection.getLatestBlockhash();

      await provider.connection.confirmTransaction({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: cancel_order_tx,
      });

      expect.fail("Order cancellation should have failed since the order is partially filled.");
    } catch (error) {
      // Check that the error thrown is related to the order being partially filled
      expect(error.toString()).to.include("OrderPartiallyFilled");
    }
  });

  it("Settles an expired order", async () => {
    const new_seed = new anchor.BN(Math.floor(Math.random() * 1000000));

    // Calculate PDA for order
    const [newOrderPDA, newOrderBump] = await PublicKey.findProgramAddressSync(
      [Buffer.from("order"), creator.publicKey.toBuffer(), new_seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    // Calculate PDA for escrow token account
    let new_escrowTokenAccount = await anchor.utils.token.associatedAddress({
      mint: vaultTokenMint,
      owner: newOrderPDA
    });

    const latestBlockHash = await provider.connection.getLatestBlockhash();

    // First, create a new order
    const new_order_tx = await program.methods.createOrder(new_seed, price, amount, new anchor.BN(0.00001)) // sub 1 second expiration
      .accountsPartial({
        creator: creator.publicKey,
        vaultTokenMint,
        priceTokenMint: usdcMint,
        creatorTokenAccount,
        order: newOrderPDA,
      })
      .signers([creator])
      .rpc();
    
    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: new_order_tx,
    }); 

    // Wait for the order to expire
    await new Promise(resolve => setTimeout(resolve, 2000));

    // Now settle the expired order
    const settle_expired_order_tx = await program.methods.settleExpiredOrder()
      .accountsPartial({
        creator: creator.publicKey,
        escrowTokenAccount: new_escrowTokenAccount,
        creatorTokenAccount,
        order: newOrderPDA,
      })
      .signers([creator])
      .rpc();

    await provider.connection.confirmTransaction({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: settle_expired_order_tx,
    }); 
    
    try {
      await program.account.order.fetch(newOrderPDA);
      expect.fail("Expired order should have been closed");
    } catch (error) {
      expect(error.toString()).to.include("Account does not exist");
    }
  });
});
