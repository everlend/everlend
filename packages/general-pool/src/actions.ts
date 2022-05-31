import { AccountLayout, MintLayout, TOKEN_PROGRAM_ID, NATIVE_MINT, Token } from '@solana/spl-token'
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js'
import BN from 'bn.js'
import {
  Pool,
  PoolBorrowAuthority,
  PoolMarket,
  UserWithdrawalRequest,
  WithdrawalRequestsState,
} from './accounts'
import { GeneralPoolsProgram } from './program'
import {
  CreateAssociatedTokenAccount,
  findAssociatedTokenAccount,
  findRegistryPoolConfigAccount,
} from '@everlend/common'
import {
  BorrowTx,
  CreatePoolTx,
  DepositTx,
  InitPoolMarketTx,
  RepayTx,
  WithdrawalRequestTx,
  WithdrawalTx,
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

export const prepareInitPoolMarketTx = async (
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
    new InitPoolMarketTx(
      { feePayer: payerPublicKey },
      {
        poolMarket: poolMarket.publicKey,
      },
    ),
  )

  return { tx, keypairs: { poolMarket } }
}

export const prepareCreatePoolTx = async (
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
    new CreatePoolTx(
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

export const prepareDepositTx = async (
  { connection, payerPublicKey }: ActionOptions,
  pool: PublicKey,
  registry: PublicKey,
  amount: BN,
  source: PublicKey,
  destination?: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { poolMarket, tokenAccount, poolMint, tokenMint },
  } = await Pool.load(connection, pool)

  const poolMarketAuthority = await GeneralPoolsProgram.findProgramAddress([poolMarket.toBuffer()])

  const tx = new Transaction()
  const registryPoolConfig = await findRegistryPoolConfigAccount(registry, pool)

  // Wrapping SOL
  let closeTokenAccountIx: TransactionInstruction
  let SOLDepositKeypair: Keypair
  let SOLDepositSource: PublicKey

  if (tokenMint.equals(NATIVE_MINT)) {
    SOLDepositKeypair = Keypair.generate()
    SOLDepositSource = SOLDepositKeypair.publicKey
    const rent = await connection.getMinimumBalanceForRentExemption(AccountLayout.span)

    const createTokenAccountIx = SystemProgram.createAccount({
      fromPubkey: payerPublicKey,
      newAccountPubkey: SOLDepositSource,
      programId: TOKEN_PROGRAM_ID,
      space: AccountLayout.span,
      lamports: amount.addn(rent).toNumber(),
    })
    const initTokenAccountIx = Token.createInitAccountInstruction(
      TOKEN_PROGRAM_ID,
      NATIVE_MINT,
      SOLDepositSource,
      payerPublicKey,
    )
    closeTokenAccountIx = Token.createCloseAccountInstruction(
      TOKEN_PROGRAM_ID,
      SOLDepositSource,
      payerPublicKey,
      payerPublicKey,
      [],
    )

    tx.add(createTokenAccountIx, initTokenAccountIx)
    source = SOLDepositSource
  }

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
    new DepositTx(
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

  closeTokenAccountIx && tx.add(closeTokenAccountIx)

  return { tx, keypairs: { SOLDepositKeypair } }
}

export const prepareWithdrawalRequestTx = async (
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

  const withdrawRequests = await WithdrawalRequestsState.getPDA(poolMarket, tokenMint)
  const withdrawalRequest = await UserWithdrawalRequest.getPDA(withdrawRequests, payerPublicKey)

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
    new WithdrawalRequestTx(
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

export const prepareWithdrawalTx = async (
  { connection, payerPublicKey }: ActionOptions,
  withdrawalRequest: PublicKey,
): Promise<ActionResult> => {
  const {
    data: { from, destination, pool },
  } = await UserWithdrawalRequest.load(connection, withdrawalRequest)

  const {
    data: { tokenMint, poolMarket, poolMint, tokenAccount },
  } = await Pool.load(connection, pool)

  const withdrawalRequests = await WithdrawalRequestsState.getPDA(poolMarket, tokenMint)
  const poolMarketAuthority = await GeneralPoolsProgram.findProgramAddress([poolMarket.toBuffer()])

  const collateralTransit = await GeneralPoolsProgram.findProgramAddress([
    Buffer.from('transit'),
    poolMarket.toBuffer(),
    poolMint.toBuffer(),
  ])

  let unwrapAccounts: UnwrapParams = undefined
  if (tokenMint.equals(NATIVE_MINT)) {
    const unwrapTokenAccount = await UserWithdrawalRequest.getUnwrapSOLPDA(withdrawalRequest)
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
    new WithdrawalTx(
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

export const prepareBorrowTx = async (
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
    new BorrowTx(
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

export const prepareRepayTx = async (
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

  const tx = new RepayTx(
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
