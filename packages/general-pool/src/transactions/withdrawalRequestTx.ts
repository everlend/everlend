import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import {
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_CLOCK_PUBKEY,
  Transaction,
  TransactionCtorFields,
  TransactionInstruction,
} from '@solana/web3.js'
import BN from 'bn.js'
import { Borsh } from '@everlend/common'
import { GeneralPoolsProgram } from '../program'

export class WithdrawalRequestTxData extends Borsh.Data<{ collateralAmount: BN }> {
  static readonly SCHEMA = this.struct([
    ['instruction', 'u8'],
    ['collateralAmount', 'u64'],
  ])

  instruction = 9
}

type WithdrawalRequestTxParams = {
  registry: PublicKey
  registryPoolConfig: PublicKey
  poolMarket: PublicKey
  pool: PublicKey
  poolMint: PublicKey
  withdrawRequests: PublicKey
  withdrawalRequest: PublicKey
  source: PublicKey
  destination: PublicKey
  tokenAccount: PublicKey
  collateralTransit: PublicKey
  collateralAmount: BN
}

export class WithdrawalRequestTx extends Transaction {
  constructor(options: TransactionCtorFields, params: WithdrawalRequestTxParams) {
    super(options)
    const { feePayer } = options
    const {
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
    } = params

    const data = WithdrawalRequestTxData.serialize({ collateralAmount })

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: registry, isSigner: false, isWritable: false },
          { pubkey: registryPoolConfig, isSigner: false, isWritable: false },
          { pubkey: poolMarket, isSigner: false, isWritable: false },
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: withdrawRequests, isSigner: false, isWritable: true },
          { pubkey: withdrawalRequest, isSigner: false, isWritable: true },
          { pubkey: source, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: tokenAccount, isSigner: false, isWritable: true },
          { pubkey: collateralTransit, isSigner: false, isWritable: true },
          { pubkey: feePayer, isSigner: true, isWritable: true },
          { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
          { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}
