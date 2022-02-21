import { GeneralPoolsProgram } from '../program'
import { AccountInfo, PublicKey } from '@solana/web3.js'
import BN from 'bn.js'
import { Buffer } from 'buffer'
import { Account, Borsh, Errors } from '@everlend/common'

type Args = {
  rentPayer: PublicKey
  source: PublicKey
  destination: PublicKey
  liquidityAmount: BN
  collateralAmount: BN
}

export class UserWithdrawRequestData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['rentPayer', 'publicKey'],
    ['source', 'publicKey'],
    ['destination', 'publicKey'],
    ['liquidityAmount', 'u64'],
    ['collateralAmount', 'u64'],
  ])

  rentPayer: PublicKey
  source: PublicKey
  destination: PublicKey
  liquidityAmount: BN
  collateralAmount: BN
}

export class UserWithdrawRequest extends Account<UserWithdrawRequestData> {
  static readonly LEN = 112

  constructor(key: PublicKey, info: AccountInfo<Buffer>) {
    super(key, info)

    if (!this.assertOwner(GeneralPoolsProgram.PUBKEY)) {
      throw Errors.ERROR_INVALID_OWNER()
    }

    this.data = UserWithdrawRequestData.deserialize(this.info.data)
  }

  static getPDA(poolMarket: PublicKey, tokenMint: PublicKey, index: BN) {
    return GeneralPoolsProgram.findProgramAddress([
      index.toArrayLike(Buffer, 'be', 8),
      Buffer.from('withdrawals'),
      new PublicKey(poolMarket).toBuffer(),
      new PublicKey(tokenMint).toBuffer(),
    ])
  }
}
