use crate::utils::{arg_keypair, arg_pubkey};
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use everlend_utils::cpi::metaplex::find_metadata_program_address;
use solana_clap_utils::input_parsers::{keypair_of, pubkey_of};
use solana_program::program_pack::Pack;
use solana_program::system_instruction;
use solana_sdk::transaction::Transaction;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_token::instruction::AuthorityType;

const ARG_TOKEN_MINT: &str = "mint";
const ARG_MULTISIG_ADDR: &str = "multisig";

#[derive(Clone, Copy)]
pub struct CreateTokenCommand;

impl<'a> ToolkitCommand<'a> for CreateTokenCommand {
    fn get_name(&self) -> &'a str {
        "create-token"
    }

    fn get_description(&self) -> &'a str {
        "Create token"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg_keypair(ARG_TOKEN_MINT, true),
            arg_pubkey(ARG_MULTISIG_ADDR, true)
                .help("Multisig address. Tokens will be minted to it and metadata authority too"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let temp_authority = Keypair::new();

        let mint = keypair_of(arg_matches, ARG_TOKEN_MINT).unwrap();
        let multisig = pubkey_of(arg_matches, ARG_MULTISIG_ADDR).unwrap();

        let decimals: u8 = 9;
        let mint_amount: u64 = 1_000_000_000;
        let metadata_name = String::from("Test Token");
        let metadata_symbol = String::from("TSTST");
        let metadata_uri = String::from("http://test.com");

        let multisig_token_acc =
            spl_associated_token_account::get_associated_token_address(&multisig, &mint.pubkey());

        let metadata_account =
            find_metadata_program_address(&mpl_token_metadata::id(), &mint.pubkey());

        let mint_rent = config
            .rpc_client
            .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)?;

        let tx = Transaction::new_with_payer(
            &[
                system_instruction::create_account(
                    &config.fee_payer.pubkey(),
                    &mint.pubkey(),
                    mint_rent,
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint.pubkey(),
                    &temp_authority.pubkey(),
                    None,
                    decimals,
                )?,
                spl_associated_token_account::create_associated_token_account(
                    &config.fee_payer.pubkey(),
                    &multisig,
                    &mint.pubkey(),
                ),
                spl_token::instruction::mint_to(
                    &spl_token::id(),
                    &mint.pubkey(),
                    &multisig_token_acc,
                    &temp_authority.pubkey(),
                    &[],
                    mint_amount,
                )?,
                mpl_token_metadata::instruction::create_metadata_accounts_v2(
                    mpl_token_metadata::id(),
                    metadata_account,
                    mint.pubkey(),
                    temp_authority.pubkey(),
                    config.fee_payer.pubkey(),
                    multisig,
                    metadata_name,
                    metadata_symbol,
                    metadata_uri,
                    None,
                    0,
                    false,
                    true,
                    None,
                    None,
                ),
                spl_token::instruction::set_authority(
                    &spl_token::id(),
                    &mint.pubkey(),
                    None,
                    AuthorityType::MintTokens, // TODO: do we need to do same for close authority?
                    &temp_authority.pubkey(),
                    &[],
                )?,
            ],
            Some(&config.fee_payer.pubkey()),
        );

        config.sign_and_send_and_confirm_transaction(
            tx,
            vec![config.fee_payer.as_ref(), &temp_authority, &mint],
        )?;

        Ok(())
    }
}
