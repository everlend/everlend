import { AccountType, GeneralPoolsProgram } from '../program'
import { AccountInfo, Connection, PublicKey } from '@solana/web3.js'
import BN from 'bn.js'
import bs58 from 'bs58'
import { Buffer } from 'buffer'
import { Account, Borsh, Errors } from '@everlend/common'

type Args = {
  accountType: AccountType
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
    ['accountType', 'u8'],
    ['pool', 'publicKey'],
    ['from', 'publicKey'],
    ['source', 'publicKey'],
    ['destination', 'publicKey'],
    ['liquidityAmount', 'u64'],
    ['collateralAmount', 'u64'],
    ['ticket', 'u64'],
  ])

  accountType: AccountType
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

  static async findMany(
    connection: Connection,
    filters: { pool?: PublicKey; from?: PublicKey } = {},
  ) {
    return (
      await GeneralPoolsProgram.getProgramAccounts(connection, {
        filters: [
          // Filter for WithdrawRequest by key
          {
            memcmp: {
              offset: 0,
              bytes: bs58.encode(Buffer.from([AccountType.WithdrawRequest])),
            },
          },
          // Filter for assigned to pool
          filters.pool && {
            memcmp: {
              offset: 1,
              bytes: new PublicKey(filters.pool).toBase58(),
            },
          },
          // Filter for assigned to from
          filters.from && {
            memcmp: {
              offset: 33,
              bytes: new PublicKey(filters.from).toBase58(),
            },
          },
        ].filter(Boolean),
      })
    )
      .map((account) => {
        try {
          return WithdrawalRequest.from(account)
        } catch (err) {}
      })
      .filter(Boolean)
  }
}
