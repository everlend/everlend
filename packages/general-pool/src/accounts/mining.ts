import { AccountInfo, PublicKey } from '@solana/web3.js'
import BN from 'bn.js'
import { Account, Borsh, Errors } from '@everlend/common'
import { RewardProgram } from '../rewardProgram'
import { Buffer } from 'buffer'

type RewardIndex = {
  rewardMint: PublicKey
  indexWithPrecision: BN
  reward: BN
}

type Args = {
  anchorId: Array<BN>
  rewardPool: PublicKey
  bump: BN
  share: BN
  owner: PublicKey
  indexes: Array<RewardIndex>
}

export class MiningData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['anchorId', ['u8', 8]],
    ['rewardPool', 'publicKey'],
    ['bump', 'u8'],
    ['share', 'u64'],
    ['owner', 'publicKey'],
    ['indexes', ['RewardIndex']],
  ])

  anchorId: Array<BN>
  rewardPool: PublicKey
  bump: BN
  share: BN
  owner: PublicKey
  indexes: Array<RewardIndex>
}

export class Mining extends Account<MiningData> {
  constructor(publicKey: PublicKey, info: AccountInfo<Buffer>) {
    super(publicKey, info)

    if (!this.assertOwner(RewardProgram.PUBKEY)) {
      throw Errors.ERROR_INVALID_OWNER()
    }

    this.data = MiningData.deserialize(this.info.data)
  }

  static getPDA(user: PublicKey, rewardPool: PublicKey) {
    return RewardProgram.findProgramAddress([
      Buffer.from('mining'),
      user.toBuffer(),
      rewardPool.toBuffer(),
    ])
  }
}
