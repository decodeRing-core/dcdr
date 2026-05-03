use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(crate) struct InitSystemRequestData {
    pub total_shares: Option<u8>,
    pub threshold: Option<u8>,
}
