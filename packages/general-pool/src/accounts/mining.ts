import { AccountInfo, PublicKey } from '@solana/web3.js'
import BN from 'bn.js'
import { Account, Borsh, Errors } from '@everlend/common'
import { RewardProgram } from '../rewardProgram'
import { Buffer } from 'buffer'
import { RewardPool } from './rewardPool'

const PRECISION = 1_000_000_000_000_000_0

type RewardIndex = {
  rewardMint: PublicKey
  indexWithPrecision: BN
  rewards: BN
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

  /**
   * Calculates user's claim amount.
   *
   * @param rewardMint Reward mint
   * @param rewardPool Reward pool
   */
  getUserClaimAmount(rewardMint: PublicKey, rewardPool: RewardPool) {
    const share = this.share

    for (const vault of rewardPool.data) {
      const rewardIndex = this.indexes.find((i) => i.rewardMint == vault.rewardMint)
      const rewardIndexI = this.indexes.indexOf(rewardIndex)

      if (vault.indexWithPrecision > rewardIndex) {
        const rewards =
          ((vault.indexWithPrecision - rewardIndex.indexWithPrecision) * share) / PRECISION

        if (rewards > 0) {
          this.indexes[rewardIndexI].rewards = rewardIndex.rewards + rewards
        }

        this.indexes[rewardIndexI].indexWithPrecision = vault.indexWithPrecision
      }

      vault.indexWithPrecision
    }

    const index = this.indexes.find((i) => i.rewardMint == rewardMint)

    return index.rewards
  }
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
