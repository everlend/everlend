use solana_program::pubkey::Pubkey;

pub struct Seeds(pub Vec<Vec<u8>>);

impl Seeds {
    pub fn as_seeds_slice(&self) -> Vec<&[u8]> {
        self.0.iter().map(Vec::as_slice).collect()
    }
}

pub trait PDA {
    fn get_raw_seeds(&self) -> Seeds;

    fn find_address(&self, program_id: &Pubkey) -> (Pubkey, u8) {
        let seeds = self.get_raw_seeds();

        Pubkey::find_program_address(&seeds.as_seeds_slice(), program_id)
    }

    fn get_signing_seeds(&self, bump: u8) -> Seeds {
        let mut seeds = self.get_raw_seeds();
        seeds.0.push(vec![bump]);
        seeds
    }
}
