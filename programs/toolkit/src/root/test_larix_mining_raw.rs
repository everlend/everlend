use crate::liquidity_mining::larix_raw_test;
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token::native_mint;
use std::str::FromStr;
use std::{thread, time};

#[derive(Clone, Copy)]
pub struct TestLarixMiningRawCommand;

impl<'a> ToolkitCommand<'a> for TestLarixMiningRawCommand {
    fn get_name(&self) -> &'a str {
        return "test-larix-mining-raw";
    }

    fn get_description(&self) -> &'a str {
        return "Test larix mining raw";
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        return vec![];
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        return vec![];
    }

    fn handle(&self, config: &Config, _arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let token_program_id =
            Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();
        let (source_sol, _) = Pubkey::find_program_address(
            &[
                &config.fee_payer.pubkey().to_bytes(),
                &spl_token::id().to_bytes(),
                &native_mint::id().to_bytes(),
            ],
            &token_program_id,
        );
        println!("source_sol {}", source_sol);
        let amount = 100_000_000; //2_500_000_000;
        let mining_account = Keypair::new();
        let collateral_transit = Keypair::new();
        let dividends_account = Keypair::new();
        let withdraw_account = Keypair::new();
        larix_raw_test::init_mining_accounts(&config, &mining_account)?;
        println!("init mining accounts finished");
        larix_raw_test::deposit_liquidity(&config, amount, &source_sol, &collateral_transit)?;

        let collateral_balance = config
            .rpc_client
            .get_token_account_balance(&collateral_transit.pubkey())
            .unwrap();

        println!("collateral_balance {:?}", collateral_balance);

        let collateral_amount = spl_token::ui_amount_to_amount(
            collateral_balance.ui_amount.unwrap(),
            collateral_balance.decimals,
        );

        println!("deposit liquidity finished");
        larix_raw_test::deposit_collateral(
            &config,
            collateral_amount,
            &mining_account.pubkey(),
            &collateral_transit.pubkey(),
        )?;
        println!("deposit collateral finished");
        thread::sleep(time::Duration::from_secs(60));
        println!("claim dividends finished");
        larix_raw_test::withdraw_collateral(
            &config,
            collateral_amount,
            &withdraw_account,
            &mining_account.pubkey(),
        )?;
        larix_raw_test::claim_mining(&config, &dividends_account, &mining_account.pubkey())?;
        println!("withdraw collateral finished");
        Ok(())
    }
}
