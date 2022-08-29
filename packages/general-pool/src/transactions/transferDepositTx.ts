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

export class TransferDepositTxData extends Borsh.Data<{ amount: BN }> {
  static readonly SCHEMA = this.struct([
    ['instruction', 'u8'],
    ['amount', 'u64'],
  ])

  instruction = 16
}

type TransferDepositTxParams = {
  pool: PublicKey
  source: PublicKey
  destination: PublicKey
  poolMarket: PublicKey
  poolMint: PublicKey
  destinationUserAuthority: PublicKey,
  rewardPool: PublicKey
  rewardAccount: PublicKey
  destinationRewardAccount: PublicKey
  config: PublicKey
  rewardProgramId: PublicKey
  amount: BN
}

export class TransferDepositTx extends Transaction {
  constructor(options: TransactionCtorFields, params: TransferDepositTxParams) {
    super(options)
    const { feePayer } = options
    const {
      pool,
      source,
      destination,
      poolMarket,
      poolMint,
      destinationUserAuthority,
      rewardPool,
      rewardAccount,
      destinationRewardAccount,
      config,
      rewardProgramId,
      amount,
    } = params

    const data = TransferDepositTxData.serialize({ amount })

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: source, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: poolMarket, isSigner: false, isWritable: false },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: feePayer, isSigner: true, isWritable: false },
          { pubkey: destinationUserAuthority, isSigner: true, isWritable: false },
          { pubkey: rewardPool, isSigner: false, isWritable: true },
          { pubkey: rewardAccount, isSigner: false, isWritable: true },
          { pubkey: destinationRewardAccount, isSigner: false, isWritable: true },
          { pubkey: config, isSigner: false, isWritable: false },
          { pubkey: rewardProgramId, isSigner: false, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}
