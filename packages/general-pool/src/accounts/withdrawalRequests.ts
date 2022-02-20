import { AccountType, GeneralPoolsProgram } from '../program'
import { AccountInfo, PublicKey } from '@solana/web3.js'
import BN from 'bn.js'
import { Buffer } from 'buffer'
import { Account, Borsh, Errors } from '@everlend/common'

type Args = {
  accountType: AccountType
  pool: PublicKey
  mint: PublicKey
  lastRequestId: BN
  lastProcessedRequestId: BN
  liquiditySupply: BN
}

export class WithdrawalRequestsData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['accountType', 'u8'],
    ['pool', 'publicKey'],
    ['mint', 'publicKey'],
    ['lastRequestId', 'u64'],
    ['lastProcessedRequestId', 'u64'],
    ['liquiditySupply', 'u64'],
  ])

  accountType: AccountType
  pool: PublicKey
  mint: PublicKey
  lastRequestId: BN
  lastProcessedRequestId: BN
  liquiditySupply: BN
}

export class WithdrawalRequests extends Account<WithdrawalRequestsData> {
  static readonly LEN = 89

  constructor(key: PublicKey, info: AccountInfo<Buffer>) {
    super(key, info)

    if (!this.assertOwner(GeneralPoolsProgram.PUBKEY)) {
      throw Errors.ERROR_INVALID_OWNER()
    }

    this.data = WithdrawalRequestsData.deserialize(this.info.data)
  }

  static getPDA(poolMarket: PublicKey, tokenMint: PublicKey) {
    return GeneralPoolsProgram.findProgramAddress([
      Buffer.from('withdrawals'),
      new PublicKey(poolMarket).toBuffer(),
      new PublicKey(tokenMint).toBuffer(),
    ])
  }
}
