import { AccountType, PoolBorrowAuthority } from '../src'
import { connection, POOL_BORROW_AUTHORITY_PUBKEY } from './utils'

describe('PoolBorrowAuthority', () => {
  beforeAll(() => {})

  describe('PoolBorrowAuthority', () => {
    test('load', async () => {
      const poolBorrowAuthority = await PoolBorrowAuthority.load(
        connection,
        POOL_BORROW_AUTHORITY_PUBKEY,
      )

      console.log(poolBorrowAuthority)
      console.log(poolBorrowAuthority.publicKey.toString())

      expect(poolBorrowAuthority.publicKey).toEqual(POOL_BORROW_AUTHORITY_PUBKEY)
      expect(poolBorrowAuthority.data.accountType).toEqual(AccountType.PoolBorrowAuthority)
    })
  })
})
