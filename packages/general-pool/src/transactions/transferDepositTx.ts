import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import {
  PublicKey,
  Transaction,
  TransactionCtorFields,
  TransactionInstruction,
} from '@solana/web3.js'
import { Borsh } from '@everlend/common'
import { GeneralPoolsProgram } from '../program'
import { RewardProgram } from '../rewardProgram'

export class TransferDepositTxData extends Borsh.Data {
  static readonly SCHEMA = this.struct([['instruction', 'u8']])

  instruction = 17
}

type TransferDepositTxParams = {
  pool: PublicKey
  source: PublicKey
  destination: PublicKey
  destinationUserAuthority: PublicKey,
  rewardPool: PublicKey
  rewardAccount: PublicKey
  destinationRewardAccount: PublicKey
  config: PublicKey
}

export class TransferDepositTx extends Transaction {
  constructor(options: TransactionCtorFields, params: TransferDepositTxParams) {
    super(options)
    const { feePayer } = options
    const {
      pool,
      source,
      destination,
      destinationUserAuthority,
      rewardPool,
      rewardAccount,
      destinationRewardAccount,
      config,
    } = params

    const data = TransferDepositTxData.serialize()

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: source, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: feePayer, isSigner: true, isWritable: false },
          { pubkey: destinationUserAuthority, isSigner: false, isWritable: false },
          { pubkey: rewardPool, isSigner: false, isWritable: true },
          { pubkey: rewardAccount, isSigner: false, isWritable: true },
          { pubkey: destinationRewardAccount, isSigner: false, isWritable: true },
          { pubkey: config, isSigner: false, isWritable: false },
          { pubkey: RewardProgram.PUBKEY, isSigner: false, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}
