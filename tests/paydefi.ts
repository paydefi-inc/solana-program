import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { Paydefi } from "../target/types/paydefi";

import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccountInstruction, getAssociatedTokenAddressSync, NATIVE_MINT } from '@solana/spl-token';
import { PublicKey, ComputeBudgetProgram, Transaction } from "@solana/web3.js";
import { jsonInfo2PoolKeys } from "@raydium-io/raydium-sdk";
import { createWrappedNativeAccount, formatAmmKeysById, sleepTime } from "./utils";

describe("Paydefi", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Paydefi as Program<Paydefi>;

  const connection = provider.connection;

  const wallet = provider.wallet;

  // WSOL
  const TOKEN_IN = new PublicKey("So11111111111111111111111111111111111111112");
  // any token to
  const TOKEN_OUT = new PublicKey("BakLYNXdUFiSKKm14HESawEpiMe8z9Tr3ynKKV1CFXZ6");
  // pool of the raydium wsol to that token out
  const POOL_ID = new PublicKey("5zWjRZZVQHpDriHKBLJeBjTdE3rUo3qGqXv1UmriBXVT");
  const MERCHANT_ADDR = new PublicKey("HQqLHpZsJREBDZNFbknDEKvgvbp5hMSK7ZxBKEQT9f4o");
  const TREASURY_ADDR = new PublicKey("HQqLHpZsJREBDZNFbknDEKvgvbp5hMSK7ZxBKEQT9f4o");
  
  const IN_AMOUNT = 500000;

  const marketInfo = {
    programId: new PublicKey("11111111111111111111111111111111"),
    serumDexProgram: new PublicKey("EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj"),
    ammProgram: new PublicKey("HWy1jotHpo6UqeQxx49dpYYdQB8wj9Qk9MdxwjLvDHB8"),
    serumMarket: new PublicKey("J1oKpxWJFHwSEHyfVfQ5eGKz2RnUUpZjYAbGBVhU8xgM"),
  }

  console.log("payer: ", wallet.publicKey.toBase58());
  console.log("merchant: ", MERCHANT_ADDR.toBase58());
  console.log("treasury: ", TREASURY_ADDR.toBase58());


  it("Is swapped!", async () => {
    let targetPoolInfo = null;
    while (true) {
      try {
        targetPoolInfo = await formatAmmKeysById(connection, POOL_ID);
        if (targetPoolInfo) {
          break; // If successful, exit the loop
        }
      } catch (error) {
        console.log(error);
        console.error('pool not found, retrying...');
      }
      await sleepTime(1000); // Wait for 1 seconds before retrying
    }
    const poolKeys = jsonInfo2PoolKeys(targetPoolInfo);

    let tx = new Transaction().add(ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }));

    const fromAta = getAssociatedTokenAddressSync(TOKEN_IN, wallet.publicKey);
    const toAta = getAssociatedTokenAddressSync(TOKEN_OUT, wallet.publicKey);
    const treasuryAta = getAssociatedTokenAddressSync(TOKEN_OUT, TREASURY_ADDR);
    const merchantAta = getAssociatedTokenAddressSync(TOKEN_OUT, MERCHANT_ADDR);

    if (TOKEN_IN.equals(NATIVE_MINT)) {

      const wrappedSolAccountInfo = await connection.getAccountInfo(fromAta);
  
        if (!wrappedSolAccountInfo) {
          tx.add(
            createAssociatedTokenAccountInstruction(
              wallet.publicKey,
              fromAta,
              wallet.publicKey,
              NATIVE_MINT
            )
          );
        }
  
        
      const transaction = createWrappedNativeAccount(wallet.publicKey, wallet.publicKey, IN_AMOUNT);
      tx.add(transaction.instructions[0]);
      tx.add(transaction.instructions[1]);

    }

    if (await connection.getAccountInfo(toAta) == null) {
      tx.add(createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        toAta,
        wallet.publicKey,
        TOKEN_OUT,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ));
    }
    if (await connection.getAccountInfo(treasuryAta) == null) {
      tx.add(createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        treasuryAta,
        TREASURY_ADDR,
        TOKEN_OUT,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ));
    }
    if (await connection.getAccountInfo(merchantAta) == null) {
      tx.add(createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        merchantAta,
        MERCHANT_ADDR,
        TOKEN_OUT,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
      ));
    }

    const swapIx = await program.methods.completeSwapPayment({
      orderId: new BN(9).toString(),
      payInToken: TOKEN_IN,
      payOutToken: TOKEN_OUT,
      payInAmount: new BN(IN_AMOUNT),
      payOutAmount: new BN(IN_AMOUNT/10),
      merchant: merchantAta,
      expiry: new BN((new Date().getTime() / 1000) + 120)
    })
      .accounts({
        payer: wallet.publicKey,
        merchant: MERCHANT_ADDR,
        treasury: TREASURY_ADDR,

        fromAta,
        toAta,
        treasuryAta,
        merchantAta,

        ammId: poolKeys.id,
        ammAuthority: poolKeys.authority,
        ammOpenOrders: poolKeys.openOrders,
        ammTargetOrders: poolKeys.targetOrders,
        poolCoinTokenAccount: poolKeys.baseVault,
        poolPcTokenAccount: poolKeys.quoteVault,
        serumProgram: marketInfo.serumDexProgram,
        serumMarket: marketInfo.serumMarket,
        serumBids: poolKeys.marketBids,
        serumAsks: poolKeys.marketAsks,
        serumEventQueue: poolKeys.marketEventQueue,
        serumCoinVault: poolKeys.marketBaseVault,
        serumPcVault: poolKeys.marketQuoteVault,
        serumVaultSigner: poolKeys.marketAuthority,
        raydiumAmmProgram: marketInfo.ammProgram,
      })
      .instruction();

    tx.add(swapIx);

    tx.feePayer = wallet.publicKey;
    tx.recentBlockhash = (await connection.getLatestBlockhash()).blockhash;

    const signedTx = await wallet.signTransaction(tx);

    const txSig = await connection.sendRawTransaction(signedTx.serialize());
    console.log("tx signature: ", txSig);
  });
});
