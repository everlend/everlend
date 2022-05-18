import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import {
  PublicKey,
  SYSVAR_CLOCK_PUBKEY,
  Transaction,
  TransactionCtorFields,
  TransactionInstruction,
} from '@solana/web3.js'
import { Borsh } from '@everlend/common'
import { GeneralPoolsProgram } from '../program'

export class WithdrawArgs extends Borsh.Data {
  static readonly SCHEMA = this.struct([['instruction', 'u8']])

  instruction = 6
}

type WithdrawParams = {
  poolMarket: PublicKey
  pool: PublicKey
  poolMarketAuthority: PublicKey
  poolMint: PublicKey
  withdrawalRequests: PublicKey
  withdrawalRequest: PublicKey
  from: PublicKey
  destination: PublicKey
  tokenAccount: PublicKey
  collateralTransit: PublicKey
}

export class Withdraw extends Transaction {
  constructor(options: TransactionCtorFields, params: WithdrawParams) {
    super(options)
    const {
      poolMarket,
      pool,
      poolMarketAuthority,
      poolMint,
      withdrawalRequests,
      withdrawalRequest,
      destination,
      tokenAccount,
      collateralTransit,
      from,
    } = params

    const data = WithdrawArgs.serialize()

    this.add(
      new TransactionInstruction({
        keys: [
          { pubkey: poolMarket, isSigner: false, isWritable: false },
          { pubkey: pool, isSigner: false, isWritable: false },
          { pubkey: poolMarketAuthority, isSigner: false, isWritable: false },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: withdrawalRequests, isSigner: false, isWritable: true },
          { pubkey: withdrawalRequest, isSigner: false, isWritable: true },
          { pubkey: destination, isSigner: false, isWritable: true },
          { pubkey: tokenAccount, isSigner: false, isWritable: true },
          { pubkey: collateralTransit, isSigner: false, isWritable: true },
          { pubkey: from, isSigner: false, isWritable: false },
          { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
          { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        ],
        programId: GeneralPoolsProgram.PUBKEY,
        data,
      }),
    )
  }
}
