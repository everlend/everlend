import { GeneralPoolsProgram } from '../program'
import { AccountInfo, PublicKey } from '@solana/web3.js'
import BN from 'bn.js'
import { Buffer } from 'buffer'
import { Account, Borsh, Errors } from '@everlend/common'

type Args = {
  pool: PublicKey
  from: PublicKey
  source: PublicKey
  destination: PublicKey
  liquidityAmount: BN
  collateralAmount: BN
  ticket: BN
}

export class WithdrawalRequestData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['pool', 'publicKey'],
    ['from', 'publicKey'],
    ['source', 'publicKey'],
    ['destination', 'publicKey'],
    ['liquidityAmount', 'u64'],
    ['collateralAmount', 'u64'],
    ['ticket', 'u64'],
  ])

  pool: PublicKey
  from: PublicKey
  source: PublicKey
  destination: PublicKey
  liquidityAmount: BN
  collateralAmount: BN
  ticket: BN
}

export class WithdrawalRequest extends Account<WithdrawalRequestData> {
  static readonly LEN = 153

  constructor(key: PublicKey, info: AccountInfo<Buffer>) {
    super(key, info)

    if (!this.assertOwner(GeneralPoolsProgram.PUBKEY)) {
      throw Errors.ERROR_INVALID_OWNER()
    }

    this.data = WithdrawalRequestData.deserialize(this.info.data)
  }

  static getPDA(withdrawalRequests: PublicKey, from: PublicKey) {
    return GeneralPoolsProgram.findProgramAddress([
      Buffer.from('withdrawal'),
      new PublicKey(withdrawalRequests).toBuffer(),
      new PublicKey(from).toBuffer(),
    ])
  }
}
