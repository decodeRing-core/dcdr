use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct InitSystemRequestData {
    pub total_shares: Option<u8>,
    pub threshold: Option<u8>,
}
