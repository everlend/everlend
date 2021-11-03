import { AccountType, LiquidityPoolsProgram } from '../program'
import { AccountInfo, PublicKey } from '@solana/web3.js'
import { Buffer } from 'buffer'
import { Account, Borsh } from '../base'
import { ERROR_INVALID_OWNER } from '../errors'

type Args = {
  accountType: AccountType
  manager: PublicKey
}
export class PoolMarketData extends Borsh.Data<Args> {
  static readonly SCHEMA = this.struct([
    ['accountType', 'u8'],
    ['manager', 'publicKey'],
  ])

  accountType: AccountType
  manager: PublicKey
}

export class PoolMarket extends Account<PoolMarketData> {
  static readonly LEN = 33

  constructor(key: PublicKey, info: AccountInfo<Buffer>) {
    super(key, info)

    if (!this.assertOwner(LiquidityPoolsProgram.PUBKEY)) {
      throw ERROR_INVALID_OWNER()
    }

    this.data = PoolMarketData.deserialize(this.info.data)
  }
}
