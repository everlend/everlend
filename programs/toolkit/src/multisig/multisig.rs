// use crate::utils::*;
// use anchor_lang::prelude::AccountMeta;
// use anchor_lang::{prelude::ToAccountMetas, Discriminator};
// use anchor_lang::{AnchorSerialize, InstructionData};
// use solana_account_decoder::UiAccountEncoding;
// use solana_client::client_error::ClientError;
// use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
// use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, MemcmpEncoding, RpcFilterType};
// use solana_program::{instruction::Instruction, pubkey::Pubkey, system_instruction};
// use solana_sdk::account::Account;
// use solana_sdk::signature::Signature;
// use solana_sdk::{
//     signature::{write_keypair_file, Keypair},
//     signer::Signer,
//     transaction::Transaction,
// };

// /// Wrapper type needed to implement `ToAccountMetas`.
// struct TransactionAccounts {
//     accounts: Vec<serum_multisig::TransactionAccount>,
//     program_id: Pubkey,
// }

// impl anchor_lang::ToAccountMetas for TransactionAccounts {
//     fn to_account_metas(&self, is_signer: Option<bool>) -> Vec<AccountMeta> {
//         assert_eq!(
//             is_signer, None,
//             "Overriding the signer is not implemented, it is not used by RequestBuilder::accounts.",
//         );
//         let mut account_metas: Vec<_> = self
//             .accounts
//             .iter()
//             .map(|tx_account| {
//                 let mut account_meta = AccountMeta::from(tx_account);
//                 account_meta.is_signer = false;
//                 account_meta
//             })
//             .collect();

//         let program_is_signer = false;
//         account_metas.push(AccountMeta::new_readonly(
//             self.program_id,
//             program_is_signer,
//         ));

//         account_metas
//     }
// }

// pub fn get_transaction_program_accounts(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
// ) -> Result<Vec<(Pubkey, Account)>, ClientError> {
//     let default_accounts = config.get_default_accounts();

//     config.rpc_client.get_program_accounts_with_config(
//         &default_accounts.multisig_program_id,
//         RpcProgramAccountsConfig {
//             filters: Some(vec![
//                 // Account parent
//                 RpcFilterType::Memcmp(Memcmp {
//                     offset: 8,
//                     bytes: MemcmpEncodedBytes::Base58(multisig_pubkey.to_string()),
//                     encoding: Some(MemcmpEncoding::Binary),
//                 }),
//             ]),
//             account_config: RpcAccountInfoConfig {
//                 encoding: Some(UiAccountEncoding::Base64Zstd),
//                 ..RpcAccountInfoConfig::default()
//             },
//             ..RpcProgramAccountsConfig::default()
//         },
//     )
// }

// pub fn get_multisig_program_address(
//     program_address: &Pubkey,
//     multisig_pubkey: &Pubkey,
// ) -> (Pubkey, u8) {
//     let seeds = [multisig_pubkey.as_ref()];
//     Pubkey::find_program_address(&seeds, program_address)
// }

// pub fn create_multisig(
//     config: &Config,
//     keypair: Option<Keypair>,
//     owners: Vec<Pubkey>,
//     threshold: u64,
// ) -> Result<(Pubkey, Pubkey), ClientError> {
//     let default_accounts = config.get_default_accounts();
//     let keypair = keypair.unwrap_or_else(Keypair::new);

//     println!("Multisig: {}", keypair.pubkey());

//     let (pda, nonce) =
//         get_multisig_program_address(&default_accounts.multisig_program_id, &keypair.pubkey());

//     let tx = Transaction::new_with_payer(
//         &[
//             system_instruction::create_account(
//                 &config.fee_payer.pubkey(),
//                 &keypair.pubkey(),
//                 config
//                     .rpc_client
//                     .get_minimum_balance_for_rent_exemption(352)?,
//                 352,
//                 &default_accounts.multisig_program_id,
//             ),
//             Instruction {
//                 program_id: default_accounts.multisig_program_id,
//                 data: serum_multisig::instruction::CreateMultisig {
//                     owners,
//                     threshold,
//                     nonce,
//                 }
//                 .data(),
//                 accounts: serum_multisig::accounts::CreateMultisig {
//                     multisig: keypair.pubkey(),
//                 }
//                 .to_account_metas(None),
//             },
//         ],
//         Some(&config.fee_payer.pubkey()),
//     );

//     config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref(), &keypair])?;
//     write_keypair_file(&keypair, &format!(".keypairs/{}.json", keypair.pubkey())).unwrap();

//     Ok((keypair.pubkey(), pda))
// }

// pub fn create_transaction(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
//     instruction: Instruction,
// ) -> Result<Pubkey, ClientError> {
//     let default_accounts = config.get_default_accounts();
//     let keypair = Keypair::new();

//     // The Multisig program expects `serum_multisig::TransactionAccount` instead
//     // of `solana_sdk::AccountMeta`.
//     let accounts: Vec<_> = instruction
//         .accounts
//         .iter()
//         .map(serum_multisig::TransactionAccount::from)
//         .collect();

//     let multisig = config.get_account_deserialize::<serum_multisig::Multisig>(multisig_pubkey)?;

//     // We are going to build a dummy version of the `serum_multisig::Transaction`,
//     // to compute its size, which we need to allocate an account for it. And to
//     // build the dummy transaction, we need to know how many owners the multisig
//     // has.
//     let dummy_tx = serum_multisig::Transaction {
//         multisig: *multisig_pubkey,
//         program_id: instruction.program_id,
//         accounts,
//         data: instruction.data.clone(),
//         signers: multisig
//             .owners
//             .iter()
//             .map(|a| a == &config.fee_payer.pubkey())
//             .collect(),
//         did_execute: false,
//         owner_set_seqno: multisig.owner_set_seqno,
//     };

//     // The space used is the serialization of the transaction itself, plus the
//     // discriminator that Anchor uses to identify the account type.
//     let mut account_bytes = serum_multisig::Transaction::discriminator().to_vec();
//     dummy_tx
//         .serialize(&mut account_bytes)
//         .expect("Failed to serialize dummy transaction.");
//     let tx_account_size = account_bytes.len();

//     let create_instruction = system_instruction::create_account(
//         &config.fee_payer.pubkey(),
//         &keypair.pubkey(),
//         config
//             .rpc_client
//             .get_minimum_balance_for_rent_exemption(tx_account_size)?,
//         tx_account_size as u64,
//         &default_accounts.multisig_program_id,
//     );

//     // The Multisig program expects `serum_multisig::TransactionAccount` instead
//     // of `solana_sdk::AccountMeta`.
//     let accounts: Vec<_> = instruction
//         .accounts
//         .iter()
//         .map(serum_multisig::TransactionAccount::from)
//         .collect();

//     let multisig_accounts = serum_multisig::accounts::CreateTransaction {
//         multisig: *multisig_pubkey,
//         transaction: keypair.pubkey(),
//         // For convenience, assume that the party that signs the proposal
//         // transaction is a member of the multisig owners, and use it as the
//         // proposer.
//         proposer: config.fee_payer.pubkey(),
//     }
//     .to_account_metas(None);
//     let multisig_ins = serum_multisig::instruction::CreateTransaction {
//         pid: instruction.program_id,
//         accs: accounts,
//         data: instruction.data,
//     };

//     let multisig_instruction = Instruction {
//         program_id: default_accounts.multisig_program_id,
//         data: multisig_ins.data(),
//         accounts: multisig_accounts,
//     };

//     let tx = Transaction::new_with_payer(
//         &[create_instruction, multisig_instruction],
//         Some(&config.fee_payer.pubkey()),
//     );
//     config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref(), &keypair])?;
//     write_keypair_file(&keypair, &format!(".keypairs/{}.json", keypair.pubkey())).unwrap();

//     Ok(keypair.pubkey())
// }

// pub fn approve(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
//     transaction_pubkey: &Pubkey,
// ) -> Result<Signature, ClientError> {
//     let default_accounts = config.get_default_accounts();

//     let tx = Transaction::new_with_payer(
//         &[Instruction {
//             program_id: default_accounts.multisig_program_id,
//             data: serum_multisig::instruction::Approve.data(),
//             accounts: serum_multisig::accounts::Approve {
//                 multisig: *multisig_pubkey,
//                 transaction: *transaction_pubkey,
//                 owner: config.fee_payer.pubkey(),
//             }
//             .to_account_metas(None),
//         }],
//         Some(&config.fee_payer.pubkey()),
//     );

//     let signature =
//         config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

//     Ok(signature)
// }

// pub fn execute_transaction(
//     config: &Config,
//     multisig_pubkey: &Pubkey,
//     transaction_pubkey: &Pubkey,
// ) -> Result<Signature, ClientError> {
//     let default_accounts = config.get_default_accounts();

//     let (pda, _) =
//         get_multisig_program_address(&default_accounts.multisig_program_id, multisig_pubkey);

//     let transaction =
//         config.get_account_deserialize::<serum_multisig::Transaction>(transaction_pubkey)?;

//     let transaction_inner_accounts = TransactionAccounts {
//         accounts: transaction.accounts,
//         program_id: transaction.program_id,
//     };

//     let mut accounts = serum_multisig::accounts::ExecuteTransaction {
//         multisig: *multisig_pubkey,
//         multisig_signer: pda,
//         transaction: *transaction_pubkey,
//     }
//     .to_account_metas(None);
//     accounts.append(&mut transaction_inner_accounts.to_account_metas(None));

//     let tx = Transaction::new_with_payer(
//         &[Instruction {
//             program_id: default_accounts.multisig_program_id,
//             data: serum_multisig::instruction::ExecuteTransaction.data(),
//             accounts,
//         }],
//         Some(&config.fee_payer.pubkey()),
//     );

//     let signature =
//         config.sign_and_send_and_confirm_transaction(tx, vec![config.fee_payer.as_ref()])?;

//     Ok(signature)
// }
