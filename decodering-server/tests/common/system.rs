use base64::{Engine, engine::general_purpose};
use rand::Rng;
use reqwest::Response;

pub fn random_shards(number_of_shards: u8) -> Vec<String> {
    let mut shards = Vec::with_capacity(number_of_shards as usize);
    for _ in 0..number_of_shards {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        shards.push(general_purpose::STANDARD.encode(bytes));
    }
    shards
}

pub async fn init_system_addr_success(
    addr: &str,
    threshold: u8,
    total_shares: u8,
) -> Result<(String, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    let resp = init_system_addr(addr, threshold, total_shares, 200).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "system-initialized");
    assert!(body["message"].is_string());
    assert!(body["data"].is_object());

    let shards = &body["data"]["shards"];
    assert!(shards.is_array());

    let root_token = &body["data"]["root_token"];
    assert!(root_token.is_string());

    let token: String = serde_json::from_value(root_token.to_owned())?;
    let shards: Vec<String> = serde_json::from_value(body["data"]["shards"].clone())?;
    Ok((token, shards))
}

pub async fn init_system_addr(
    addr: &str,
    threshold: u8,
    total_shares: u8,
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/system/init"))
        .json(&serde_json::json!({
            "total_shares": threshold,
            "threshold": total_shares,
            "plugins_credentials": {}
        }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn unlock_system_addr_success(
    addr: &str,
    shards: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = unlock_system_addr(addr, shards, 200).await?;
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "system-unlocked");
    assert_eq!(body["message"], "System unlocked");

    Ok(())
}

pub async fn unlock_system_addr(
    addr: &str,
    shards: &[String],
    expected_status: u16,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/system/unlock"))
        .json(&serde_json::json!({ "shards": shards }))
        .send()
        .await?;
    assert_eq!(resp.status(), expected_status);
    Ok(resp)
}

pub async fn status_system_addr(
    addr: &str,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let resp = reqwest::Client::new()
        .get(format!("http://{addr}/system/status"))
        .json(&serde_json::json!({}))
        .send()
        .await?;
    assert_eq!(resp.status(), 200);
    Ok(resp)
}

pub async fn status_system_addr_unlocked(
    addr: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = status_system_addr(addr).await?;

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["osl_version"], "1.0.0");
    assert_eq!(body["status"], "system-status");
    assert_eq!(body["message"], "System status");
    assert_eq!(body["data"]["initialized"], true);
    assert_eq!(body["data"]["unlocked"], true);

    Ok(())
}
