import BN from "bn.js";
import assert from "assert";
import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Keypair } from '@solana/web3.js';
import {
  createMint,
  createAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
  mintTo,
} from '@solana/spl-token';


// Anchor
import * as anchor from '@coral-xyz/anchor';
import { Program, BN } from '@coral-xyz/anchor';
import { TransferSol } from '../target/types/transfer_sol';
import type { TransferSol } from "../target/types/transfer_sol";


describe('transfer-sol', () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.TransferSol as anchor.Program<TransferSol>;
  
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.env();
  // @ts-ignore
  const payer = provider.wallet.payer;

  const program = anchor.workspace.TransferSol as Program<TransferSol>;

  it('Trasnfer SPL Token to random taker', async () => {
    // Generate keypairs for the new accounts
    const to = new Keypair();
    // Create a Payment struct similar to Rust struct
    const payment = {
      orderId: "order123", // Example order ID
      payInToken: TOKEN_PROGRAM_ID, // Replace with actual token mint address
      payOutToken: TOKEN_PROGRAM_ID, // Replace with actual token mint address
      payInAmount: new anchor.BN(1000), // Example amount in smallest denomination
      payOutAmount: new anchor.BN(900), // Example amount in smallest denomination
      merchant: to.publicKey, // Replace with merchant's public key
      expiry: new anchor.BN(Math.floor(Date.now() / 1000) + 3600) // Unix timestamp for expiry (1 hour from now)
    };

    // Create a new mint and initialize it
    const mint = await createMint(
      provider.connection,
      payer,
      provider.wallet.publicKey,
      null,
      0
    );

    // Create associated token accounts for the new accounts
    const payerAta = await createAssociatedTokenAccount(
      provider.connection,
      payer,
      mint,
      provider.wallet.publicKey
    );
    const toAta = await createAssociatedTokenAccount(
      provider.connection,
      payer,
      mint,
      to.publicKey
    );
  
    // Mint tokens to the 'from' associated token account
    const mintAmount = 1000;
    const mintedSignature = await mintTo(
      provider.connection,
      payer,
      mint,
      payerAta,
      provider.wallet.publicKey,
      mintAmount
    );

    // Send transaction
    // const transferAmount = new BN(500);
    const transferredSignature = await program.methods
      .completeTransferPayment(payment)
      .accounts({
        payer: payer.publicKey,
        fromAta: payerAta,
        toAta: toAta,
        treasuryAta: toAta,
        tokenProgram: TOKEN_PROGRAM_ID
      })
      .signers([payer])
      .rpc();


    console.log('---------------------------------------');
    console.log('Trasnfer SPL Token to random taker');
    console.log('---------------------------------------');
    console.log('payer/from =>', payer.publicKey);
    console.log('to =>', to.publicKey);
    console.log('mintedSignature =>', mintedSignature);
    console.log('transferredSignature =>', transferredSignature);

    const toTokenAccount = await provider.connection.getTokenAccountBalance(toAta);
    // assert.strictEqual(
    //   toTokenAccount.value.uiAmount,
    //   transferAmount.toNumber(),
    //   "The 'to' token account should have the transferred tokens"
    // );
  });
});
