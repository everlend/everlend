import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import {
  PublicKey,
  Transaction,
  TransactionCtorFields,
  TransactionInstruction,
} from '@solana/web3.js'
import BN from 'bn.js'
import { Borsh } from '@everlend/common'
import { GeneralPoolsProgram } from '../program'

export class WithdrawRequestArgs extends Borsh.Data<{ amount: BN }> {
  static readonly SCHEMA = this.struct([
    ['instruction', 'u8'],
    ['amount', 'u64'],
  ])

  instruction = 9
}

type WithdrawRequestParams = {
  poolMarket: PublicKey
  pool: PublicKey
  withdrawRequests: PublicKey
  source: PublicKey
  destination: PublicKey
  tokenAccount: PublicKey
  collateralTransit: PublicKey
  poolMint: PublicKey
  amount: BN
}

export class WithdrawRequest extends Transaction {
  constructor(options: TransactionCtorFields, params: WithdrawRequestParams) {
    super(options)
    const { feePayer } = options
    const {
      poolMarket,
      pool,
      withdrawRequests,
      source,
      destination,
      tokenAccount,
      collateralTransit,
      poolMint,
      amount,
    } = params

    const data = WithdrawRequestArgs.serialize({ amount })

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: poolMarket, isSigner: false, isWritable: false },
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: withdrawRequests, isSigner: false, isWritable: true },
          { pubkey: source, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: tokenAccount, isSigner: false, isWritable: true },
          { pubkey: collateralTransit, isSigner: false, isWritable: true },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: feePayer, isSigner: true, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}
