use std::{collections::HashMap, sync::OnceLock};

use reqwest::Client;

use crate::aws::error::AwsError;

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

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_iam_arn_role() {
        let parsed = parse_iam_arn("arn:aws:iam::123456789012:role/MyRole").unwrap();
        assert_eq!(parsed.account_id, "123456789012");
        assert!(matches!(parsed.kind, ArnKind::Role));
        assert_eq!(parsed.name, "MyRole");
    }

    #[test]
    fn parse_iam_arn_user() {
        let parsed = parse_iam_arn("arn:aws:iam::123456789012:user/Alice").unwrap();
        assert_eq!(parsed.account_id, "123456789012");
        assert!(matches!(parsed.kind, ArnKind::User));
        assert_eq!(parsed.name, "Alice");
    }

    #[test]
    fn parse_iam_arn_role_with_path() {
        let parsed = parse_iam_arn("arn:aws:iam::123456789012:role/path/to/MyRole").unwrap();
        assert_eq!(parsed.name, "path/to/MyRole");
    }

    #[test]
    fn parse_iam_arn_rejects_non_arn_prefix() {
        assert!(parse_iam_arn("notarn:aws:iam::123456789012:role/MyRole").is_none());
    }

    #[test]
    fn parse_iam_arn_rejects_wrong_partition() {
        assert!(parse_iam_arn("arn:gov:iam::123456789012:role/MyRole").is_none());
    }

    #[test]
    fn parse_iam_arn_rejects_wrong_service() {
        assert!(parse_iam_arn("arn:aws:sts::123456789012:role/MyRole").is_none());
    }

    #[test]
    fn parse_iam_arn_rejects_unknown_resource_type() {
        assert!(parse_iam_arn("arn:aws:iam::123456789012:group/Admins").is_none());
    }

    #[test]
    fn parse_iam_arn_rejects_too_few_parts() {
        assert!(parse_iam_arn("arn:aws:iam").is_none());
    }

    #[test]
    fn parse_iam_arn_empty_string() {
        assert!(parse_iam_arn("").is_none());
    }

    #[test]
    fn normalize_arn_role_passthrough() {
        let out = normalize_arn("arn:aws:iam::123456789012:role/MyRole").unwrap();
        assert_eq!(out, "arn:aws:iam::123456789012:role/MyRole");
    }

    #[test]
    fn normalize_arn_user_passthrough() {
        let out = normalize_arn("arn:aws:iam::123456789012:user/Bob").unwrap();
        assert_eq!(out, "arn:aws:iam::123456789012:user/Bob");
    }

    #[test]
    fn normalize_arn_rewrites_assumed_role() {
        let out =
            normalize_arn("arn:aws:sts::123456789012:assumed-role/MyRole/session-name").unwrap();
        assert_eq!(out, "arn:aws:iam::123456789012:role/MyRole");
    }

    #[test]
    fn normalize_arn_assumed_role_without_session() {
        let out = normalize_arn("arn:aws:sts::123456789012:assumed-role/MyRole").unwrap();
        assert_eq!(out, "arn:aws:iam::123456789012:role/MyRole");
    }

    #[test]
    fn normalize_arn_rejects_federated_user() {
        assert!(normalize_arn("arn:aws:sts::123456789012:federated-user/Alice").is_none());
    }

    #[test]
    fn normalize_arn_rejects_garbage() {
        assert!(normalize_arn("not-an-arn").is_none());
    }

    #[test]
    fn validate_sts_request_accepts_valid() {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_owned(),
            "AWS4-HMAC-SHA256 ...".to_owned(),
        );
        let result = validate_sts_request(
            "POST",
            "https://sts.amazonaws.com/",
            "Action=GetCallerIdentity&Version=2011-06-15",
            &headers,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_sts_request_accepts_lowercase_authorization_header() {
        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_owned(),
            "AWS4-HMAC-SHA256 ...".to_owned(),
        );
        let result = validate_sts_request(
            "POST",
            "https://sts.amazonaws.com/",
            "Action=GetCallerIdentity",
            &headers,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_sts_request_accepts_lowercase_method() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_owned(), "x".to_owned());
        let result = validate_sts_request(
            "post",
            "https://sts.amazonaws.com/",
            "Action=GetCallerIdentity",
            &headers,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn validate_sts_request_rejects_wrong_url() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_owned(), "x".to_owned());
        let result = validate_sts_request(
            "POST",
            "https://sts.us-east-1.amazonaws.com/",
            "Action=GetCallerIdentity",
            &headers,
        );
        assert!(matches!(result, Err(AwsError::InvalidStsUrl)));
    }

    #[test]
    fn validate_sts_request_rejects_wrong_action() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_owned(), "x".to_owned());
        let result = validate_sts_request(
            "POST",
            "https://sts.amazonaws.com/",
            "Action=AssumeRole",
            &headers,
        );
        assert!(matches!(result, Err(AwsError::InvalidStsAction)));
    }

    #[test]
    fn validate_sts_request_rejects_wrong_method() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_owned(), "x".to_owned());
        let result = validate_sts_request(
            "GET",
            "https://sts.amazonaws.com/",
            "Action=GetCallerIdentity",
            &headers,
        );
        assert!(matches!(result, Err(AwsError::InvalidStsAction)));
    }

    #[test]
    fn validate_sts_request_rejects_missing_authorization() {
        let headers = HashMap::new();
        let result = validate_sts_request(
            "POST",
            "https://sts.amazonaws.com/",
            "Action=GetCallerIdentity",
            &headers,
        );
        assert!(matches!(result, Err(AwsError::InvalidInput)));
    }

    #[test]
    fn validate_sts_request_rejects_empty_body() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_owned(), "x".to_owned());
        let result = validate_sts_request("POST", "https://sts.amazonaws.com/", "", &headers);
        assert!(matches!(result, Err(AwsError::InvalidStsAction)));
    }

    #[test]
    fn parse_get_caller_identity_response_full() {
        let xml = "
        <GetCallerIdentityResponse>
            <GetCallerIdentityResult>
                <Account>123456789012</Account>
                <Arn>arn:aws:iam::123456789012:user/Alice</Arn>
                <UserId>AIDACKCEVSQ6C2EXAMPLE</UserId>
            </GetCallerIdentityResult>
        </GetCallerIdentityResponse>
    ";
        let identity = parse_get_caller_identity_response(xml).unwrap();
        assert_eq!(identity.account, "123456789012");
        assert_eq!(identity.arn, "arn:aws:iam::123456789012:user/Alice");
        assert_eq!(identity.user_id, "AIDACKCEVSQ6C2EXAMPLE");
    }

    #[test]
    fn parse_get_caller_identity_response_trims_whitespace() {
        let xml = "<Account>  123  </Account><Arn>  arn:x  </Arn><UserId>  u  </UserId>";
        let identity = parse_get_caller_identity_response(xml).unwrap();
        assert_eq!(identity.account, "123");
        assert_eq!(identity.arn, "arn:x");
        assert_eq!(identity.user_id, "u");
    }

    #[test]
    fn parse_get_caller_identity_response_missing_account() {
        let xml = "<Arn>arn:x</Arn><UserId>u</UserId>";
        assert!(parse_get_caller_identity_response(xml).is_none());
    }

    #[test]
    fn parse_get_caller_identity_response_missing_arn() {
        let xml = "<Account>123</Account><UserId>u</UserId>";
        assert!(parse_get_caller_identity_response(xml).is_none());
    }

    #[test]
    fn parse_get_caller_identity_response_missing_user_id() {
        let xml = "<Account>123</Account><Arn>arn:x</Arn>";
        assert!(parse_get_caller_identity_response(xml).is_none());
    }

    #[test]
    fn parse_get_caller_identity_response_empty() {
        assert!(parse_get_caller_identity_response("").is_none());
    }

    #[test]
    fn parse_get_caller_identity_response_unclosed_tag() {
        let xml = "<Account>123<Arn>arn</Arn><UserId>u</UserId>";
        assert!(parse_get_caller_identity_response(xml).is_none());
    }
}
