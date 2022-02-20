import { PublicKey } from '@solana/web3.js'
import { Program } from '@everlend/common'

export enum AccountType {
  Uninitialized = 0,
  PoolMarket = 1,
  Pool = 2,
  PoolBorrowAuthority = 3,
  WithdrawRequests = 4,
}

export class GeneralPoolsProgram extends Program {
  static readonly PUBKEY = new PublicKey('EzDzLfEtcDHfKduQ7pu36rUM2FWfDCLkifp2pzcmGM3p')
}
