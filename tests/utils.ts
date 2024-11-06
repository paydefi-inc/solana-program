import { PublicKey, Connection, Transaction, SystemProgram } from "@solana/web3.js";
import { Liquidity, Market, LIQUIDITY_STATE_LAYOUT_V4, MARKET_STATE_LAYOUT_V3, SPL_MINT_LAYOUT, TOKEN_PROGRAM_ID } from '@raydium-io/raydium-sdk';
import { ASSOCIATED_TOKEN_PROGRAM_ID, createSyncNativeInstruction, getAssociatedTokenAddressSync, NATIVE_MINT } from "@solana/spl-token";

export const sleepTime = async (ms: number) => {
    return new Promise(resolve => setTimeout(resolve, ms))
}

export const formatAmmKeysById = async (connection: Connection, id: PublicKey) => {
    const account = await connection.getAccountInfo(id);

    if (account === null) throw Error(' get id info error ')
    const info = LIQUIDITY_STATE_LAYOUT_V4.decode(account.data)

    const marketId = info.marketId
    const marketAccount = await connection.getAccountInfo(marketId)
    if (marketAccount === null) throw Error(' get market info error')
    const marketInfo = MARKET_STATE_LAYOUT_V3.decode(marketAccount.data)

    const lpMint = info.lpMint
    const lpMintAccount = await connection.getAccountInfo(lpMint)
    if (lpMintAccount === null) throw Error(' get lp mint info error')
    const lpMintInfo = SPL_MINT_LAYOUT.decode(lpMintAccount.data)

    return {
        id: id.toString(),
        baseMint: info.baseMint.toString(),
        quoteMint: info.quoteMint.toString(),
        lpMint: info.lpMint.toString(),
        baseDecimals: info.baseDecimal.toNumber(),
        quoteDecimals: info.quoteDecimal.toNumber(),
        lpDecimals: lpMintInfo.decimals,
        version: 4,
        programId: account.owner.toString(),
        authority: Liquidity.getAssociatedAuthority({ programId: account.owner }).publicKey.toString(),
        openOrders: info.openOrders.toString(),
        targetOrders: info.targetOrders.toString(),
        baseVault: info.baseVault.toString(),
        quoteVault: info.quoteVault.toString(),
        withdrawQueue: info.withdrawQueue.toString(),
        lpVault: info.lpVault.toString(),
        marketVersion: 3,
        marketProgramId: info.marketProgramId.toString(),
        marketId: info.marketId.toString(),
        marketAuthority: Market.getAssociatedAuthority({ programId: info.marketProgramId, marketId: info.marketId }).publicKey.toString(),
        marketBaseVault: marketInfo.baseVault.toString(),
        marketQuoteVault: marketInfo.quoteVault.toString(),
        marketBids: marketInfo.bids.toString(),
        marketAsks: marketInfo.asks.toString(),
        marketEventQueue: marketInfo.eventQueue.toString(),
        lookupTableAccount: PublicKey.default.toString()
    }
}

export const createWrappedNativeAccount = (
    payer: PublicKey,
    owner: PublicKey,
    amount: number
): Transaction => {
    const associatedToken = getAssociatedTokenAddressSync(
        NATIVE_MINT,
        owner,
        true,
        TOKEN_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const transaction = new Transaction().add(
        SystemProgram.transfer({
            fromPubkey: payer,
            toPubkey: associatedToken,
            lamports: amount,
        }),
        createSyncNativeInstruction(associatedToken, TOKEN_PROGRAM_ID)
    );

    return transaction;
}
