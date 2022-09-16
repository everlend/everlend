use crate::accounts_config::{CollateralPoolAccounts, TokenAccounts};
use crate::helpers::{
    create_collateral_pool, create_general_pool, create_income_pool, create_pool_borrow_authority,
    create_token_oracle, create_transit, PoolPubkeys,
};
use crate::utils::{arg_multiple, get_asset_maps, spl_create_associated_token_account};
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_liquidity_oracle::state::DistributionArray;
use solana_client::client_error::ClientError;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;

pub struct CreateTokenAccountsCommand;

const ARG_MINTS: &str = "mints";

impl<'a> ToolkitCommand<'a> for CreateTokenAccountsCommand {
    fn get_name(&self) -> &'a str {
        "create-token-accounts"
    }

    fn get_description(&self) -> &'a str {
        "Create a new token accounts"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![arg_multiple(ARG_MINTS, true).short("m")]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();
        let required_mints: Vec<_> = arg_matches.values_of(ARG_MINTS).unwrap().collect();

        let payer_pubkey = config.fee_payer.pubkey();
        let default_accounts = config.get_default_accounts();
        let mut initialiazed_accounts = config.get_initialized_accounts();

        let (mint_map, collateral_mint_map) = get_asset_maps(default_accounts);

        let mut distribution = DistributionArray::default();
        distribution[0] = 0;
        distribution[1] = 0;
        distribution[2] = 0;

        println!("Prepare borrow authority");
        let (depositor_authority, _) = &everlend_utils::find_program_address(
            &everlend_depositor::id(),
            &initialiazed_accounts.depositor,
        );

        for key in required_mints {
            let mint = mint_map.get(key).unwrap();
            let collateral_mints: Vec<(Pubkey, Pubkey)> = collateral_mint_map
                .get(key)
                .unwrap()
                .iter()
                .zip(initialiazed_accounts.mm_pool_markets.iter())
                .filter_map(|(collateral_mint, mm_pool_market_pubkey)| {
                    collateral_mint.map(|coll_mint| (coll_mint, *mm_pool_market_pubkey))
                })
                .collect();

            println!("General pool");
            let (general_pool_pubkey, general_pool_token_account, general_pool_mint) =
                create_general_pool(config, &initialiazed_accounts.general_pool_market, mint)?;

            let token_account = get_associated_token_address(&payer_pubkey, mint);
            println!("Payer token account: {:?}", token_account);
            // let pool_account = get_associated_token_address(&payer_pubkey, &general_pool_mint);
            let pool_account =
                spl_create_associated_token_account(config, &payer_pubkey, &general_pool_mint)
                    .unwrap_or_else(|_| {
                        get_associated_token_address(&payer_pubkey, &general_pool_mint)
                    });
            println!("Payer pool account: {:?}", pool_account);

            println!("Income pool");
            let (income_pool_pubkey, income_pool_token_account) =
                create_income_pool(config, &initialiazed_accounts.income_pool_market, mint)?;

            // MM Pools
            let mm_pool_pubkeys = collateral_mints
                .iter()
                .map(|(collateral_mint, mm_pool_market_pubkey)| {
                    println!("MM Pool: {}", collateral_mint);
                    if collateral_mint.eq(&Pubkey::default()) {
                        // We can't skip cuz of mm pools is indexed
                        Ok(PoolPubkeys {
                            pool: Pubkey::default(),
                            token_account: Pubkey::default(),
                        })
                    } else {
                        create_collateral_pool(config, mm_pool_market_pubkey, collateral_mint)
                    }
                })
                .collect::<Result<Vec<PoolPubkeys>, ClientError>>()?;

            create_token_oracle(
                config,
                &initialiazed_accounts.liquidity_oracle,
                mint,
                &distribution,
            )?;

            // Transit accounts
            let liquidity_transit_pubkey =
                create_transit(config, &initialiazed_accounts.depositor, mint, None)?;

            // Reserve
            println!("Reserve transit");
            create_transit(
                config,
                &initialiazed_accounts.depositor,
                mint,
                Some("reserve".to_string()),
            )?;

            println!("Collateral transits");
            collateral_mints
                .iter()
                .filter(|(pk, _)| !pk.eq(&Pubkey::default()))
                .map(|(collateral_mint, _mm_pool_market_pubkey)| {
                    create_transit(
                        config,
                        &initialiazed_accounts.depositor,
                        collateral_mint,
                        None,
                    )
                })
                .collect::<Result<Vec<Pubkey>, ClientError>>()?;

            let collateral_pools = collateral_mints
                .iter()
                .zip(mm_pool_pubkeys)
                .map(|((collateral_mint, _mm_pool_market_pubkey), pubkeys)| {
                    CollateralPoolAccounts {
                        pool: pubkeys.pool,
                        pool_token_account: pubkeys.token_account,
                        token_mint: *collateral_mint,
                    }
                })
                .collect();

            // Borrow authorities
            create_pool_borrow_authority(
                config,
                &initialiazed_accounts.general_pool_market,
                &general_pool_pubkey,
                depositor_authority,
                10_000, // 100%
            )?;

            initialiazed_accounts.token_accounts.insert(
                key.to_string(),
                TokenAccounts {
                    mint: *mint,
                    liquidity_token_account: token_account,
                    collateral_token_account: pool_account,
                    general_pool: general_pool_pubkey,
                    general_pool_token_account,
                    general_pool_mint,
                    income_pool: income_pool_pubkey,
                    income_pool_token_account,
                    mm_pools: Vec::new(),
                    mining_accounts: Vec::new(),
                    collateral_pools,
                    liquidity_transit: liquidity_transit_pubkey,
                    port_finance_obligation_account: Pubkey::default(),
                },
            );
        }

        initialiazed_accounts
            .save(&format!("accounts.{}.yaml", config.network))
            .unwrap();

        Ok(())
    }
}
