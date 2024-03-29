mod utils;

mod depositor {
    mod create_transit;
    mod deposit;
    mod init;
    mod reset_rebalancing;
    mod start_rebalancing;
    mod withdraw;
}

mod collateral_pool {
    mod borrow;
    mod create_pool;
    mod create_pool_borrow_authority;
    mod delete_pool_borrow_authority;
    mod deposit;
    mod init_pool_market;
    mod repay;
    mod update_pool_borrow_authority;
    mod withdraw;
}

mod general_pool {
    mod borrow;
    mod cancel_withdraw_request;
    mod create_pool;
    mod create_pool_borrow_authority;
    mod delete_pool_borrow_authority;
    mod deposit;
    mod init_pool_market;
    mod repay;
    mod transfer_deposit;
    mod update_manager;
    mod update_pool_borrow_authority;
    mod update_pool_config;
    mod withdraw;
    mod withdraw_request;
}

mod liquidity_oracle {
    mod create_token_oracle;
    mod init_liquidity_oracle;
    mod update_liquidity_oracle;
    mod update_token_oracle;
}

mod income_pools {
    mod create_pool;
    mod deposit;
    mod init_pool_market;
    mod withdraw;
}

mod registry {
    mod init;
    mod update_registry;
}

mod rewards {
    mod add_vault;
    mod claim;
    mod deposit_mining;
    mod fill_vault;
    mod initialize_mining;
    mod initialize_pool;
    mod withdraw_mining;
}
