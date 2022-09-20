import { AccountInfo, PublicKey } from "@solana/web3.js"
import { Account, Borsh, Errors } from '@everlend/common'
import BN from 'bn.js'
import { RewardProgram } from '../rewardProgram'

type Args = {
  anchorId: Array<BN>
  authority: PublicKey
}

export class RootAccountData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['anchorId', ['u8', 8]],
    ['authority', 'publicKey'],
  ])

  anchorId: Array<BN>
  authority: PublicKey
}

export class RootAccount extends Account<RootAccountData> {
  constructor(publicKey: PublicKey, info: AccountInfo<Buffer>) {
    super(publicKey, info)

    if (!this.assertOwner(RewardProgram.PUBKEY)) {
      throw Errors.ERROR_INVALID_OWNER()
    }

    this.data = RootAccountData.deserialize(this.info.data)
  }
}
