#![cfg(test)]

use crate::contract::is_difficulty_correct;
use soroban_sdk::{BytesN, Env};

#[test]
fn test_is_difficulty_correct() {
    // Hash: BytesN<32>(0, 0, 0, 56, 38, 109, 57, 170, 142, 27, 143, 23, 149, 96, 45, 107, 234, 142, 67, 208, 5, 191, 37, 66, 121, 193, 142, 94, 226, 70, 117, 5)
    // Hash in Hex format: "00000038266d39aa8e1b8f1795602d6bea8e43d005bf254279c18e5ee2467505"
    let e: Env = Env::default();
    let bytes: BytesN<32> = BytesN::from_array(
        &e,
        &[
            0, 0, 0, 56, 38, 109, 57, 170, 142, 27, 143, 23, 149, 96, 45, 107, 234, 142, 67, 208,
            5, 191, 37, 66, 121, 193, 142, 94, 226, 70, 117, 5,
        ],
    );

    assert_eq!(is_difficulty_correct(&bytes, &6), true);
}
