use aws_config::BehaviorVersion;
use aws_credential_types::provider::ProvideCredentials;
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::time::SystemTime;

pub async fn generate_aws_sig(region: &str) -> Result<(), Box<dyn Error>> {
    let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let creds = config
        .credentials_provider()
        .ok_or("no credentials provider")?
        .provide_credentials()
        .await?;

    let body = "Action=GetCallerIdentity&Version=2011-06-15";
    let url = "https://sts.amazonaws.com/";

    let identity = creds.into();
    let signing_params = v4::SigningParams::builder()
        .identity(&identity)
        .region(region)
        .name("sts")
        .time(SystemTime::now())
        .settings(SigningSettings::default())
        .build()?
        .into();

    let headers = vec![
        ("host", "sts.amazonaws.com"),
        ("content-type", "application/x-www-form-urlencoded"),
    ];

    let signable = SignableRequest::new(
        "POST",
        url,
        headers.iter().map(|(k, v)| (*k, *v)),
        SignableBody::Bytes(body.as_bytes()),
    )?;

    let (instructions, _sig) = sign(signable, &signing_params)?.into_parts();

    let mut all_headers: HashMap<String, String> = headers
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    for header in instructions.headers() {
        all_headers.insert(header.0.to_owned(), header.1.to_owned());
    }

    let payload = serde_json::json!({
        "method":  "POST",
        "url":     url,
        "body":    body,
        "headers": all_headers,
    });

    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{payload:#}")?;
    Ok(())
}
