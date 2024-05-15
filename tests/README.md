# End-to-end tests

## e2e_btc_test

Tests "sunny day scenario" of a BTC **deposit** flow with Vault.

BTC is locked in Vault, while GGX:Alice wallet gets KBTC.

1. We start BTC and GGX.
2. Alice calls ggx::tx().oracle().feed_value().
3. Start vault. It connects to both BTC and GGX.
4. Mine 50 BTC to address BTC:Alice and mine some blocks on top to confirm.
5. Deposit 500k sat to GGX:
5.1 Alice sends ggx::tx().issue().request_issue(500k sat) and waits for event RequestIssue - it contains Vault's BTC pubkey, which should be used to deposit BTC.
5.2 Alice sends 500k sat from BTC:Alice to Vault wallet.
6. We check that Alice's KBTC (wrapped BTC) is more than 0, less than 500k sat - some fees are deducted. Amount depends on BTC price (sent to GGX via oracle, step 2).

## e2e_ibc_test

Tests "sunny day scenario" that users can deposit ERT asset from Cosmos to GGX via Hermes IBC channel.
Then, tests that users can withdraw ERT asset from GGX back to Cosmos over same channel.

1. We start GGX and Cosmos.
2. We start Hermes, which connects to GGX and Cosmos and establishes bidirectional IBC channel.
3. Alice calls ggx::tx().assets().force_create() to create asset with id=666 (ERT).
4. Deposit:
   - Alice uses hermes to deposit 999000ERT from Cosmos:Alice to GGX:Bob.
   - We check that Cosmos:Alice balance decreased by 999000ERT.
   - We check that GGX:Bob balance increased by 999000ERT.
5. Withdraw:
   - Alice uses hermes to withdraw 500000ERT from GGX:Bob to Cosmos:Alice.
   - We check that GGX:Bob balance decreased by 500000ERT.
   - We check that Cosmos:Alice balance increased by 500000ERT.

## e2e_dex_test

Tests common DEX functionality. We create 2 assets, then:
1. Alice has only asset A, Bob has only asset B.
2. Alice creates order to sell A for B.
3. We list open orders, and see it.
4. Bob fulfills this order.
5. Alice creates 3 more orders.
6. We list open orders, we see them.
7. Alice cancels 1 order.
8. We list open orders, we can see that cancelled order is no longer open.
