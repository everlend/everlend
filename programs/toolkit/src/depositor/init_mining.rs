use crate::liquidity_mining::larix_liquidity_miner::LarixLiquidityMiner;
use crate::liquidity_mining::port_liquidity_miner::PortLiquidityMiner;
use crate::liquidity_mining::quarry_liquidity_miner::QuarryLiquidityMiner;
use crate::liquidity_mining::{execute_init_mining_accounts, save_mining_accounts, LiquidityMiner};
use crate::utils::arg;
use crate::{Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_utils::integrations::{MoneyMarket, StakingMoneyMarket};
use solana_clap_utils::input_parsers::{pubkey_of, value_of};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

const ARG_STAKING_MM: &str = "staking-money-market";
const ARG_TOKEN: &str = "token";
const ARG_SUB_REWARD_MINT: &str = "sub-reward-mint";

#[derive(Clone, Copy)]
pub struct InitMiningCommand;

impl<'a> ToolkitCommand<'a> for InitMiningCommand {
    fn get_name(&self) -> &'a str {
        "init-mining"
    }

    fn get_description(&self) -> &'a str {
        "Init mining"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg(ARG_STAKING_MM, true)
                .value_name("NUMBER")
                .help("Money market index"),
            arg(ARG_TOKEN, true)
                .short("t")
                .value_name("TOKEN")
                .help("Token"),
            arg(ARG_SUB_REWARD_MINT, false)
                .short("m")
                .value_name("REWARD_MINT")
                .help("Sub reward token mint"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let staking_money_market = value_of::<usize>(arg_matches, ARG_STAKING_MM).unwrap();
        let staking_money_market = StakingMoneyMarket::from(staking_money_market);
        let token = value_of::<String>(arg_matches, ARG_TOKEN).unwrap();
        let sub_reward_mint = pubkey_of(arg_matches, ARG_SUB_REWARD_MINT);

        let liquidity_miner_option: Option<Box<dyn LiquidityMiner>> = match staking_money_market {
            StakingMoneyMarket::PortFinance => Some(Box::new(PortLiquidityMiner {})),
            StakingMoneyMarket::Larix => Some(Box::new(LarixLiquidityMiner {})),
            StakingMoneyMarket::Quarry => Some(Box::new(QuarryLiquidityMiner {})),
            _ => None,
        };

        if liquidity_miner_option.is_none() {
            return Err(anyhow::anyhow!("Wrong staking money market"));
        }
        let liquidity_miner = liquidity_miner_option.unwrap();
        let mut mining_pubkey = liquidity_miner.get_mining_pubkey(config, &token);

        if mining_pubkey.eq(&Pubkey::default()) {
            let new_mining_account = Keypair::new();
            mining_pubkey = new_mining_account.pubkey();
            liquidity_miner.create_mining_account(
                config,
                &token,
                &new_mining_account,
                sub_reward_mint,
            )?;
        };

        let pubkeys = liquidity_miner.get_pubkeys(config, &token);
        let mining_type =
            liquidity_miner.get_mining_type(config, &token, mining_pubkey, sub_reward_mint);

        execute_init_mining_accounts(config, &pubkeys.unwrap(), mining_type)?;

        let money_market = match staking_money_market {
            StakingMoneyMarket::Larix => MoneyMarket::Larix,
            StakingMoneyMarket::Solend => MoneyMarket::Solend,
            _ => MoneyMarket::PortFinance,
        };

        save_mining_accounts(config, &token, money_market)?;

        Ok(())
    }
}
