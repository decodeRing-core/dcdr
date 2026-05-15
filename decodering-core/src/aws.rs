use std::{collections::HashMap, sync::OnceLock};

use reqwest::Client;

use crate::error::AwsError;

pub struct RoleArn {
    pub account_id: String,
    pub role_name: String,
}

pub fn parse_role_arn(arn: &str) -> Option<RoleArn> {
    // Accepts: arn:aws:iam::123456789012:role/payments-service
    let mut parts = arn.split(':');
    if parts.next()? != "arn" {
        return None;
    }
    if parts.next()? != "aws" {
        return None;
    }
    if parts.next()? != "iam" {
        return None;
    }
    let _region = parts.next()?; // empty for IAM
    let account_id = parts.next()?.to_owned();
    let resource = parts.next()?;
    let role_name = resource.strip_prefix("role/")?.to_owned();
    Some(RoleArn {
        account_id,
        role_name,
    })
}

/// Canonicalize: store role ARNs in their iam:role form, regardless of
/// whether the input came as an iam: or sts:assumed-role variant.
pub fn normalize_role_arn(arn: &str) -> Option<String> {
    // Already iam:role form
    if let Some(parts) = parse_role_arn(arn) {
        return Some(format!(
            "arn:aws:iam::{}:role/{}",
            parts.account_id, parts.role_name
        ));
    }
    // sts:assumed-role form: arn:aws:sts::123:assumed-role/payments-service/i-abc123
    let mut parts = arn.split(':');
    if parts.next()? != "arn" {
        return None;
    }
    if parts.next()? != "aws" {
        return None;
    }
    if parts.next()? != "sts" {
        return None;
    }
    let _region = parts.next()?;
    let account_id = parts.next()?;
    let resource = parts.next()?;
    let mut res_parts = resource.split('/');
    if res_parts.next()? != "assumed-role" {
        return None;
    }
    let role_name = res_parts.next()?;
    let _session = res_parts.next();
    Some(format!("arn:aws:iam::{account_id}:role/{role_name}"))
}

pub fn validate_sts_request(
    method: &str,
    url: &str,
    body: &str,
    headers: &HashMap<String, String>,
) -> Result<(), AwsError> {
    if url != "https://sts.amazonaws.com/" {
        return Err(AwsError::InvalidStsUrl);
    }
    if !body.contains("Action=GetCallerIdentity") {
        return Err(AwsError::InvalidStsAction);
    }
    if method.to_uppercase() != "POST" {
        return Err(AwsError::InvalidStsAction);
    }
    if !headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("Authorization"))
    {
        return Err(AwsError::InvalidInput);
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct StsIdentity {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

fn http_client() -> Result<&'static Client, reqwest::Error> {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    if let Some(client) = CLIENT.get() {
        return Ok(client);
    }
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    Ok(CLIENT.get_or_init(|| client))
}

pub async fn call_sts_get_caller_identity(
    url: &str,
    body: String,
    headers: HashMap<String, String>,
) -> Result<StsIdentity, AwsError> {
    let client = http_client().map_err(|_| AwsError::StsUnavailableClient)?;
    let mut builder = client.post(url).body(body.clone());
    for (name, value) in &headers {
        builder = builder.header(name, value);
    }

    let resp = builder.send().await.map_err(|_| AwsError::StsUnreachable)?;
    if !resp.status().is_success() {
        return Err(AwsError::StsRejected);
    }

    let body = resp.text().await.map_err(|_| AwsError::StsUnreachable)?;

    parse_get_caller_identity_response(&body).ok_or(AwsError::StsRejected)
}

fn parse_get_caller_identity_response(xml: &str) -> Option<StsIdentity> {
    let extract = |tag: &str| -> Option<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        let start = xml.find(&open)? + open.len();
        let rest = xml.get(start..)?;
        let end = rest.find(&close)?;
        rest.get(..end).map(|s| s.trim().to_owned())
    };
    Some(StsIdentity {
        account: extract("Account")?,
        arn: extract("Arn")?,
        user_id: extract("UserId")?,
    })
}
