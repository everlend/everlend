import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { PublicKey } from '@solana/web3.js'
import { RegistryProgram } from './registryProgram'

export const findAssociatedTokenAccount = async (owner: PublicKey, mint: PublicKey) => {
  return (
    await PublicKey.findProgramAddress(
      [owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
      ASSOCIATED_TOKEN_PROGRAM_ID,
    )
  )[0]
}

export const findRegistryPoolConfigAccount =
async (registry: PublicKey, pool: PublicKey) => {
  return (
    await PublicKey.findProgramAddress(
      [Buffer.from('config', 'utf-8'), registry.toBuffer(), pool.toBuffer()],
      RegistryProgram.PUBKEY,
    )
  )[0]
}
