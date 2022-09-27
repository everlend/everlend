import { PublicKey, AccountInfo } from '@solana/web3.js'
import { RewardProgram, RewardsAccountType } from '../rewardProgram'
import { Account, Borsh, Errors } from '@everlend/common'
import BN from 'bn.js'
import { GeneralPoolsProgram } from '../program'
import { Buffer } from 'buffer'

type RewardVault = {
  bump: BN
  rewardMint: PublicKey
  indexWithPrecision: BN
  feeAccount: PublicKey
}

type Args = {
  accountType: RewardsAccountType
  rewardsRoot: PublicKey
  bump: BN
  liquidityMint: PublicKey
  totalShare: BN
  vaults: Array<RewardVault>
  depositAuthority: PublicKey
}

export class RewardPoolData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['accountType', 'u8'],
    ['rewardsRoot', 'publicKey'],
    ['bump', 'u8'],
    ['liquidityMint', 'publicKey'],
    ['totalShare', 'u64'],
    ['vaults', ['RewardVault']],
    ['depositAuthority', 'publicKey'],
  ])

  accountType: RewardsAccountType
  rewardsRoot: PublicKey
  bump: BN
  liquidityMint: PublicKey
  totalShare: BN
  vaults: Array<RewardVault>
  depositAuthority: PublicKey
}

export class RewardPool extends Account<RewardPoolData> {
  constructor(publicKey: PublicKey, info: AccountInfo<Buffer>) {
    super(publicKey, info)

    if (!this.assertOwner(RewardProgram.PUBKEY)) {
      throw Errors.ERROR_INVALID_OWNER()
    }

    this.data = RewardPool.deserialize(this.info.data)
  }

  static getVaultPDA(rewardMint: PublicKey, rewardPool: PublicKey) {
    return GeneralPoolsProgram.findProgramAddress([
      Buffer.from('vault'),
      rewardPool.toBuffer(),
      rewardMint.toBuffer(),
    ])
  }
}
