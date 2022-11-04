use crate::utils::{ arg };
use crate::{utils::Config, ToolkitCommand};
use clap::{Arg, ArgMatches};
use solana_clap_utils::input_parsers::{ value_of };
use sha2::{Digest, Sha256};

const SIGHASH_STATE_NAMESPACE: &str = "state";
const SIGHASH_GLOBAL_NAMESPACE: &str = "global";
const ARG_IX_NAME: &str = "name";
const ARG_IS_GLOBAL_NAMESPACE: &str  = "is_global";


#[derive(Clone, Copy)]
pub struct AnchorEncodeCommand;

impl<'a> ToolkitCommand<'a> for AnchorEncodeCommand {
    fn get_name(&self) -> &'a str {
        "anchor-encode"
    }

    fn get_description(&self) -> &'a str {
        "Anchor encode"
    }

    fn get_args(&self) -> Vec<Arg<'a, 'a>> {
        vec![
            arg(ARG_IX_NAME, true),
            arg(ARG_IS_GLOBAL_NAMESPACE, true)
                .help("Function name"),
        ]
    }

    fn get_subcommands(&self) -> Vec<Box<dyn ToolkitCommand<'a>>> {
        vec![]
    }

    fn handle(&self, _config: &Config, arg_matches: Option<&ArgMatches>) -> anyhow::Result<()> {
        let arg_matches = arg_matches.unwrap();

        let ix_name: String = value_of(arg_matches, ARG_IX_NAME).unwrap();
        let is_global_namespace: bool = value_of(arg_matches, ARG_IS_GLOBAL_NAMESPACE).unwrap();

        let mut hasher = Sha256::new();
        let arg = if is_global_namespace {
            format!("{}:{}", SIGHASH_GLOBAL_NAMESPACE, ix_name)
        } else {
            format!("{}:{}", SIGHASH_STATE_NAMESPACE, ix_name)
        };
        hasher.update(arg);
        let result = hasher.finalize();
        let mut array = [0u8; 8];
        for (&x, p) in result.iter().zip(array.iter_mut()) {
            *p = x;
        }

        println!("{:?}", array);

        Ok(())
    }
}
