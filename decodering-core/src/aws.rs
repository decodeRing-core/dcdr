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
