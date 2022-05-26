import { AccountLayout, MintLayout, TOKEN_PROGRAM_ID, NATIVE_MINT } from '@solana/spl-token'
import { Connection, Keypair, PublicKey, SystemProgram, Transaction } from '@solana/web3.js'
import BN from 'bn.js'
import {
  Pool,
  PoolBorrowAuthority,
  PoolMarket,
  WithdrawalRequest,
  WithdrawalRequests,
} from './accounts'
import { GeneralPoolsProgram } from './program'
import {
  CreateAssociatedTokenAccount,
  findAssociatedTokenAccount,
  findRegistryPoolConfigAccount,
} from '@everlend/common'
import {
  Borrow,
  CreatePool,
  Deposit,
  InitPoolMarket,
  Repay,
  WithdrawRequest,
  Withdraw,
  UnwrapParams,
} from './transactions'
import { Buffer } from 'buffer'

export type ActionResult = {
  tx: Transaction
  keypairs?: Record<string, Keypair>
}

export type ActionOptions = {
  connection: Connection
  payerPublicKey: PublicKey
}

export const initPoolMarket = async (
  { connection, payerPublicKey }: ActionOptions,
  poolMarket = Keypair.generate(),
): Promise<ActionResult> => {
  const lamports = await connection.getMinimumBalanceForRentExemption(PoolMarket.LEN)

  const tx = new Transaction()
  tx.add(
    SystemProgram.createAccount({
      fromPubkey: payerPublicKey,
      newAccountPubkey: poolMarket.publicKey,
      lamports,
      space: PoolMarket.LEN,
      programId: GeneralPoolsProgram.PUBKEY,
    }),
  )
  tx.add(
    new InitPoolMarket(
      { feePayer: payerPublicKey },
      {
        poolMarket: poolMarket.publicKey,
      },
    ),
  )

  return { tx, keypairs: { poolMarket } }
}

export const createPool = async (
  { connection, payerPublicKey }: ActionOptions,
  poolMarket: PublicKey,
  tokenMint: PublicKey,
  tokenAccount = Keypair.generate(),
  poolMint = Keypair.generate(),
): Promise<ActionResult> => {
  const tokenAccountlamports = await connection.getMinimumBalanceForRentExemption(
    AccountLayout.span,
  )
  const poolMintlamports = await connection.getMinimumBalanceForRentExemption(MintLayout.span)

  const tx = new Transaction()
  tx.add(
    SystemProgram.createAccount({
      fromPubkey: payerPublicKey,
      newAccountPubkey: tokenAccount.publicKey,
      lamports: tokenAccountlamports,
      space: AccountLayout.span,
      programId: TOKEN_PROGRAM_ID,
    }),
  )
  tx.add(
    SystemProgram.createAccount({
      fromPubkey: payerPublicKey,
      newAccountPubkey: poolMint.publicKey,
      lamports: poolMintlamports,
      space: MintLayout.span,
      programId: TOKEN_PROGRAM_ID,
    }),
  )

  const poolPubkey = await Pool.getPDA(poolMarket, tokenMint)
  const poolMarketAuthority = await GeneralPoolsProgram.findProgramAddress([poolMarket.toBuffer()])

  tx.add(
    new CreatePool(
      { feePayer: payerPublicKey },
      {
        poolMarket,
        pool: poolPubkey,
        tokenMint,
        tokenAccount: tokenAccount.publicKey,
        poolMint: poolMint.publicKey,
        poolMarketAuthority,
      },
    ),
  )

  return { tx, keypairs: { tokenAccount, poolMint } }
}

export const deposit = async (
  { connection, payerPublicKey }: ActionOptions,
  pool: PublicKey,
  registry: PublicKey,
  amount: BN,
  source: PublicKey,
  destination?: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { poolMarket, tokenAccount, poolMint },
  } = await Pool.load(connection, pool)

  const poolMarketAuthority = await GeneralPoolsProgram.findProgramAddress([poolMarket.toBuffer()])

  const tx = new Transaction()
  const registryPoolConfig = await findRegistryPoolConfigAccount(registry, pool)

  // Create destination account for pool mint if doesn't exist
  destination = destination ?? (await findAssociatedTokenAccount(payerPublicKey, poolMint))
  !(await connection.getAccountInfo(destination)) &&
    tx.add(
      new CreateAssociatedTokenAccount(
        { feePayer: payerPublicKey },
        {
          associatedTokenAddress: destination,
          tokenMint: poolMint,
        },
      ),
    )

  tx.add(
    new Deposit(
      { feePayer: payerPublicKey },
      {
        registryPoolConfig,
        registry,
        poolMarket,
        pool,
        source,
        destination,
        tokenAccount,
        poolMint,
        poolMarketAuthority,
        amount,
      },
    ),
  )

  return { tx }
}

export const withdrawRequest = async (
  { connection, payerPublicKey }: ActionOptions,
  pool: PublicKey,
  registry: PublicKey,
  collateralAmount: BN,
  source: PublicKey,
  destination?: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { tokenMint, poolMarket, tokenAccount, poolMint },
  } = await Pool.load(connection, pool)

  const withdrawRequests = await WithdrawalRequests.getPDA(poolMarket, tokenMint)
  const withdrawalRequest = await WithdrawalRequest.getPDA(withdrawRequests, payerPublicKey)

  const collateralTransit = await GeneralPoolsProgram.findProgramAddress([
    Buffer.from('transit'),
    poolMarket.toBuffer(),
    poolMint.toBuffer(),
  ])

  const tx = new Transaction()

  const registryPoolConfig = await findRegistryPoolConfigAccount(registry, pool)
  // Create destination account for token mint if doesn't exist
  destination = destination ?? (await findAssociatedTokenAccount(payerPublicKey, tokenMint))
  !(await connection.getAccountInfo(destination)) &&
    tx.add(
      new CreateAssociatedTokenAccount(
        { feePayer: payerPublicKey },
        {
          associatedTokenAddress: destination,
          tokenMint,
        },
      ),
    )

  tx.add(
    new WithdrawRequest(
      { feePayer: payerPublicKey },
      {
        registry,
        registryPoolConfig,
        poolMarket,
        pool,
        withdrawRequests,
        withdrawalRequest,
        source,
        destination,
        tokenAccount,
        collateralTransit,
        poolMint,
        collateralAmount,
      },
    ),
  )

  return { tx }
}

export const withdraw = async (
  { connection, payerPublicKey }: ActionOptions,
  withdrawalRequest: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { from, destination, pool },
  } = await WithdrawalRequest.load(connection, withdrawalRequest)

  const {
    data: { tokenMint, poolMarket, poolMint, tokenAccount },
  } = await Pool.load(connection, pool)

  const withdrawalRequests = await WithdrawalRequests.getPDA(poolMarket, tokenMint)
  const poolMarketAuthority = await GeneralPoolsProgram.findProgramAddress([poolMarket.toBuffer()])

  const collateralTransit = await GeneralPoolsProgram.findProgramAddress([
    Buffer.from('transit'),
    poolMarket.toBuffer(),
    poolMint.toBuffer(),
  ])

  let unwrapAccounts: UnwrapParams = undefined
  if (tokenMint.equals(NATIVE_MINT)) {
    const unwrapTokenAccount = await WithdrawalRequest.getUnwrapSOLPDA(withdrawalRequest)
    unwrapAccounts = {
      tokenMint: tokenMint,
      unwrapTokenAccount: unwrapTokenAccount,
      signer: payerPublicKey,
    }
  }

  const tx = new Transaction()

  // Create destination account for token mint if doesn't exist
  !(await connection.getAccountInfo(destination)) &&
    tx.add(
      new CreateAssociatedTokenAccount(
        { feePayer: payerPublicKey },
        {
          associatedTokenAddress: destination,
          tokenMint,
        },
      ),
    )

  tx.add(
    new Withdraw(
      { feePayer: payerPublicKey },
      {
        poolMarket,
        pool,
        poolMarketAuthority,
        poolMint,
        withdrawalRequests,
        withdrawalRequest,
        destination,
        tokenAccount,
        collateralTransit,
        from,
        unwrapAccounts,
      },
    ),
  )

  return { tx }
}

export const borrow = async (
  { connection, payerPublicKey }: ActionOptions,
  pool: PublicKey,
  amount: BN,
  destination?: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { poolMarket, tokenAccount, tokenMint },
  } = await Pool.load(connection, pool)

  const poolMarketAuthority = await GeneralPoolsProgram.findProgramAddress([poolMarket.toBuffer()])
  const poolBorrowAuthority = await PoolBorrowAuthority.getPDA(pool, payerPublicKey)

  const tx = new Transaction()

  // Create destination account for token mint if doesn't exist
  destination = destination ?? (await findAssociatedTokenAccount(payerPublicKey, tokenMint))
  !(await connection.getAccountInfo(destination)) &&
    tx.add(
      new CreateAssociatedTokenAccount(
        { feePayer: payerPublicKey },
        {
          associatedTokenAddress: destination,
          tokenMint: tokenMint,
        },
      ),
    )

  tx.add(
    new Borrow(
      { feePayer: payerPublicKey },
      {
        poolMarket,
        pool,
        poolBorrowAuthority,
        destination,
        tokenAccount,
        poolMarketAuthority,
        amount,
      },
    ),
  )

  return { tx }
}

export const repay = async (
  { connection, payerPublicKey }: ActionOptions,
  pool: PublicKey,
  amount: BN,
  interestAmount: BN,
  source: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { poolMarket, tokenAccount },
  } = await Pool.load(connection, pool)

  const poolBorrowAuthority = await PoolBorrowAuthority.getPDA(pool, payerPublicKey)

  const tx = new Repay(
    { feePayer: payerPublicKey },
    {
      poolMarket,
      pool,
      poolBorrowAuthority,
      source,
      tokenAccount,
      amount,
      interestAmount,
    },
  )

  return { tx }
}
