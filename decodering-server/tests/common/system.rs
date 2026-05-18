use base64::{Engine, engine::general_purpose};
use rand::Rng;

pub fn random_shards(number_of_shards: u8) -> Vec<String> {
    let mut shards = Vec::with_capacity(number_of_shards as usize);
    for _ in 0..number_of_shards {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        shards.push(general_purpose::STANDARD.encode(bytes));
    }
    shards
}
