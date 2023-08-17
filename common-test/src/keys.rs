#![allow(dead_code)]

use libp2p::identity::{secp256k1, Keypair};

pub const TEST_KEYPAIR_A: &str = "dc404f7ed2d3cdb65b536e8d561255c84658e83775ee790ff46bf4d77690b0fe";
pub const TEST_KEYPAIR_B: &str = "9c0cd57a01ee12338915b42bf6232a386e467dcdbe172facd94e4623ffc9096c";
pub const TEST_KEYPAIR_C: &str = "136db36b07faf276b5b9ed11f6d977d0176005435ccee50eba6f4fba998bd3f8";
pub const TEST_KEYPAIR_D: &str = "888f2dc4e8eda1e52743d9ac4e4593c80e049a88bdaf418aa723ebddf03532ea";
pub const TEST_KEYPAIR_E: &str = "54f75484a46219ad284c6e07d9f2de8990eba0ca0cd8ffd4f84b7b6ba9f837bd";

pub fn secp256k1_keypair(key: &str) -> Keypair {
    let raw_key = hex::decode(key).expect("key to be valid");
    let secret_key = secp256k1::SecretKey::try_from_bytes(raw_key).unwrap();
    secp256k1::Keypair::from(secret_key).into()
}
