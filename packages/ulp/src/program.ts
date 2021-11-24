import { PublicKey } from '@solana/web3.js'
import { Program } from '@everlend/common'

export enum AccountType {
  Uninitialized = 0,
  PoolMarket = 1,
  Pool = 2,
  PoolBorrowAuthority = 3,
}

export class LiquidityPoolsProgram extends Program {
  static readonly PUBKEY = new PublicKey('sFPqhpo9CJ4sCMPwsaZwmC25WERMW27x1M1be3DY5BM')
}
