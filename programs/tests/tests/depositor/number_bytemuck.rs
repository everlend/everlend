use solana_program_test::*;

use anchor_lang::__private::bytemuck;
use borsh::{BorshDeserialize, BorshSerialize};
use jet_proto_math::Number;
use rand::Rng;

use crate::utils::*;

#[tokio::test]
async fn number_bytemuck() {
    let mut arr: [u8; 24] = [0u8; 24];

    for i in 0..23 {
        arr[i] = rand::thread_rng().gen_range(0..255);
    }

    let number_1 = Number::from_bits(arr);
    let number_2 = bytemuck::from_bytes::<Number>(&arr);

    assert_eq!(number_1, *number_2);
}