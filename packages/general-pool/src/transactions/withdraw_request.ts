import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import {
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionCtorFields,
  TransactionInstruction,
} from '@solana/web3.js'
import BN from 'bn.js'
import { Borsh } from '@everlend/common'
import { GeneralPoolsProgram } from '../program'

export class WithdrawRequestArgs extends Borsh.Data<{ collateralAmount: BN }> {
  static readonly SCHEMA = this.struct([
    ['instruction', 'u8'],
    ['collateralAmount', 'u64'],
  ])

  instruction = 9
}

type WithdrawRequestParams = {
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

export class WithdrawRequest extends Transaction {
  constructor(options: TransactionCtorFields, params: WithdrawRequestParams) {
    super(options)
    const { feePayer } = options
    const {
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

    const data = WithdrawRequestArgs.serialize({ collateralAmount })

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: poolMarket, isSigner: false, isWritable: false },
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: withdrawRequests, isSigner: false, isWritable: true },
          { pubkey: withdrawalRequest, isSigner: false, isWritable: true },
          { pubkey: source, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: tokenAccount, isSigner: false, isWritable: true },
          { pubkey: collateralTransit, isSigner: false, isWritable: true },
          { pubkey: feePayer, isSigner: true, isWritable: false },
          { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
          { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}
