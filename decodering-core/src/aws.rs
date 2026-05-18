use std::{collections::HashMap, sync::OnceLock};

use reqwest::Client;

use crate::error::AwsError;

#[derive(Debug)]
pub struct ParsedArn {
    pub account_id: String,
    pub kind: ArnKind,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub enum ArnKind {
    Role,
    User,
}

pub fn parse_iam_arn(arn: &str) -> Option<ParsedArn> {
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
    let _region = parts.next()?;
    let account_id = parts.next()?.to_owned();
    let resource = parts.next()?;

    if let Some(name) = resource.strip_prefix("role/") {
        return Some(ParsedArn {
            account_id,
            kind: ArnKind::Role,
            name: name.to_owned(),
        });
    }
    if let Some(name) = resource.strip_prefix("user/") {
        return Some(ParsedArn {
            account_id,
            kind: ArnKind::User,
            name: name.to_owned(),
        });
    }
    None
}

pub fn normalize_arn(arn: &str) -> Option<String> {
    // Already in iam:role or iam:user form
    if let Some(parsed) = parse_iam_arn(arn) {
        let resource = match parsed.kind {
            ArnKind::Role => "role",
            ArnKind::User => "user",
        };
        return Some(format!(
            "arn:aws:iam::{}:{}/{}",
            parsed.account_id, resource, parsed.name
        ));
    }

    // sts:assumed-role form — rewrite to iam:role
    if let Some((account_id, role_name)) = parse_assumed_role_arn(arn) {
        return Some(format!("arn:aws:iam::{account_id}:role/{role_name}"));
    }

    None
}

fn parse_assumed_role_arn(arn: &str) -> Option<(String, String)> {
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
    let account_id = parts.next()?.to_owned();
    let resource = parts.next()?;
    let mut res_parts = resource.split('/');
    if res_parts.next()? != "assumed-role" {
        return None;
    }
    let role_name = res_parts.next()?.to_owned();
    let _session = res_parts.next();
    Some((account_id, role_name))
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
