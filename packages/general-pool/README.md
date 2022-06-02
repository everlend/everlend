# Everlend general-pool package

Everlend is a lending aggregator and optimizer. Get the best rates on your liquidity loans, always.

The SDK allows interacting with Everlend general pools.

## Installation

### Yarn

`$ yarn add @everlend/general-pool`

### NPM

`npm install @everlend/general-pool`

## Usage

### Find pools
```js
import { Pool } from '@everlend/general-pool';

const pools = await Pool.findMany(connection, {
    poolMarket,
});
```

### Find user's withdrawal requests
```js
import { UserWithdrawalRequest } from '@everlend/general-pool';

const userWithdrawalRequests = await UserWithdrawalRequest.findMany(connection, {
    from,
});
```

### Prepare a deposit transaction
```js
import { prepareDepositTx } from '@everlend/general-pool';

const depositTx = await prepareDepositTx(
  { connection, payerPublicKey, },
  pool,
  registry,
  amount,
  source,
  destination,
);
```

### Prepare a withdrawal request transaction
```js
import { prepareWithdrawalRequestTx } from '@everlend/general-pool';

const withdrawalRequestTx = await prepareWithdrawalRequestTx(
  { connection, payerPublicKey, },
  pool,
  registry,
  amount,
  source,
  destination,
);
```

### Prepare a withdrawal transaction
```js
import { prepareWithdrawalTx } from '@everlend/general-pool';

const withdrawalTx = await prepareWithdrawalTx(
  {
    connection,
    payerPublicKey,
  },
  withdrawalRequest,
);
```

## Pool market public keys

**Mainnet:** DzGDoJHdzUANM7P7V25t5nxqbvzRcHDmdhY51V6WNiXC

**Devnet:** 4yC3cUWXQmoyyybfnENpxo33hiNxUNa1YAmmuxz93WAJ

## Registry public keys

**Mainnet:** UaqUGgMvVzUZLthLHC9uuuBzgw5Ldesich94Wu5pMJg

**Devnet:** 6KCHtgSGR2WDE3aqrqSJppHRGVPgy9fHDX5XD8VZgb61
