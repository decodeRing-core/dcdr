use serde_json::Value;
use serde_json::json;
use utoipa::ToSchema;
use utoipa::openapi::ComponentsBuilder;
use utoipa::openapi::ContentBuilder;
use utoipa::openapi::HttpMethod;
use utoipa::openapi::InfoBuilder;
use utoipa::openapi::OpenApiBuilder;
use utoipa::openapi::PathItem;
use utoipa::openapi::PathsBuilder;
use utoipa::openapi::Ref;
use utoipa::openapi::RefOr;
use utoipa::openapi::Required;
use utoipa::openapi::Response;
use utoipa::openapi::Type;
use utoipa::openapi::path::OperationBuilder;
use utoipa::openapi::path::ParameterBuilder;
use utoipa::openapi::path::ParameterIn;
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::ObjectBuilder;

use decodering_core::plugin::orchestrator::BackendCapabilities;
use decodering_core::plugin::osl_contract::Capability;
use decodering_core::plugin::osl_contract::DescribeOutput;

use crate::handlers::osl::payload::DeleteSecretRequestData;
use crate::handlers::osl::payload::DescribeSecretRequestData;
use crate::handlers::osl::payload::DestroySecretRequestData;
use crate::handlers::osl::payload::GetSecretRequestData;
use crate::handlers::osl::payload::IsTaintedSecretRequestData;
use crate::handlers::osl::payload::ListSecretRequestData;
use crate::handlers::osl::payload::Options;
use crate::handlers::osl::payload::PutSecretRequestData;
use crate::handlers::osl::payload::RestoreSecretRequestData;
use crate::handlers::osl::payload::Store;
use crate::handlers::osl::payload::TaintSecretRequestData;
use crate::handlers::osl::payload::UntaintSecretRequestData;
use crate::handlers::osl::response::ApiCapabilitiesResponse;
use crate::handlers::osl::response::ApiDeleteSecretResponse;
use crate::handlers::osl::response::ApiDescribeSecretResponse;
use crate::handlers::osl::response::ApiDestroySecretResponse;
use crate::handlers::osl::response::ApiGetSecretMetadataResponse;
use crate::handlers::osl::response::ApiGetSecretResponse;
use crate::handlers::osl::response::ApiIsTaintedSecretResponse;
use crate::handlers::osl::response::ApiListAppsResponse;
use crate::handlers::osl::response::ApiListBackendsResponse;
use crate::handlers::osl::response::ApiListSecretResponse;
use crate::handlers::osl::response::ApiPutSecretResponse;
use crate::handlers::osl::response::ApiRestoreSecretResponse;
use crate::handlers::osl::response::ApiTaintSecretResponse;
use crate::handlers::response::ApiErrorBody;
use crate::handlers::response::ApiResponse;
use crate::handlers::response::SuccessStatus;
use crate::handlers::system::payloads::InitSystemData;
use crate::handlers::system::payloads::PluginConfigData;
use crate::handlers::system::payloads::UnlockData;
use crate::handlers::system::response::ApiInitSystemResponse;

use decodering_core::domain::PrincipalCredentialKind;
use decodering_core::domain::PrincipalKind;

use crate::handlers::app::payload::AppGrantData;
use crate::handlers::app::payload::AuthActivationData;
use crate::handlers::app::payload::AuthChallengeData;
use crate::handlers::app::payload::AuthUserData;
use crate::handlers::app::payload::CreateAppData;
use crate::handlers::app::payload::CreateAppUserData;
use crate::handlers::app::payload::ListAppsData as AppListAppsData;
use crate::handlers::app::payload::RevokeAppData;
use crate::handlers::app::response::ApiAuthAppUserResponse;
use crate::handlers::app::response::ApiAuthChallengeResponse;
use crate::handlers::app::response::ApiCreateAppGrantResponse;
use crate::handlers::app::response::ApiCreateAppResponse;
use crate::handlers::app::response::ApiCreateAppUserResponse;
use crate::handlers::app::response::ApiDeleteAppGrantResponse;

fn json_body_ex(
    schema: &str,
    example: Option<Value>,
) -> utoipa::openapi::request_body::RequestBody {
    let mut content = ContentBuilder::new().schema(Some(RefOr::Ref(Ref::from_schema_name(schema))));
    if let Some(ex) = example {
        content = content.example(Some(ex));
    }
    RequestBodyBuilder::new()
        .content("application/json", content.build())
        .required(Some(Required::True))
        .build()
}

fn json_response(desc: &str, schema: &str, example: Option<Value>) -> Response {
    let mut content = ContentBuilder::new().schema(Some(RefOr::Ref(Ref::from_schema_name(schema))));
    if let Some(ex) = example {
        content = content.example(Some(ex));
    }
    ResponseBuilder::new()
        .description(desc)
        .content("application/json", content.build())
        .build()
}

fn post(
    req: &str,
    resp_schema: &str,
    tag: &str,
    success_example: Option<Value>,
    errors: Vec<(&str, &str, Value)>,
) -> PathItem {
    post_ex(req, None, resp_schema, tag, success_example, errors)
}

fn post_ex(
    req: &str,
    req_example: Option<Value>,
    resp_schema: &str,
    tag: &str,
    success_example: Option<Value>,
    errors: Vec<(&str, &str, Value)>,
) -> PathItem {
    let mut op = OperationBuilder::new()
        .request_body(Some(json_body_ex(req, req_example)))
        .response(
            "200",
            json_response("Success", resp_schema, success_example),
        )
        .tag(tag);

    for (status, desc, example) in errors {
        op = op.response(status, json_response(desc, resp_schema, Some(example)));
    }

    PathItem::new(HttpMethod::Post, op.build())
}

fn get(
    resp_schema: &str,
    tag: &str,
    success_example: Option<Value>,
    errors: Vec<(&str, &str, Value)>,
) -> PathItem {
    let mut op = OperationBuilder::new()
        .response(
            "200",
            json_response("Success", resp_schema, success_example),
        )
        .tag(tag);

    for (status, desc, example) in errors {
        op = op.response(status, json_response(desc, resp_schema, Some(example)));
    }

    PathItem::new(HttpMethod::Get, op.build())
}

fn get_with_query(
    query_params: Vec<(&str, bool, &str)>,
    resp_schema: &str,
    tag: &str,
    success_example: Option<Value>,
    errors: Vec<(&str, &str, Value)>,
) -> PathItem {
    let mut op = OperationBuilder::new()
        .response(
            "200",
            json_response("Success", resp_schema, success_example),
        )
        .tag(tag);

    for (name, required, desc) in query_params {
        let param = ParameterBuilder::new()
            .name(name)
            .parameter_in(ParameterIn::Query)
            .required(if required {
                Required::True
            } else {
                Required::False
            })
            .description(Some(desc))
            .schema(Some(ObjectBuilder::new().schema_type(Type::String).build()))
            .build();
        op = op.parameter(param);
    }

    for (status, desc, example) in errors {
        op = op.response(status, json_response(desc, resp_schema, Some(example)));
    }

    PathItem::new(HttpMethod::Get, op.build())
}

fn ok_example(data: Value) -> Value {
    let mut envelope = serde_json::Map::new();
    envelope.insert("osl_version".to_owned(), json!("1.0.0"));
    envelope.insert("status".to_owned(), json!("operation-completed"));
    envelope.insert("message".to_owned(), json!("Operation completed"));
    envelope.insert("data".to_owned(), data);
    Value::Object(envelope)
}

fn err_example(detail: &str) -> Value {
    json!({
        "osl_version": "1.0.0",
        "error": {
            "code": "operation-failed",
            "message": "Operation failed.",
            "detail": detail
        }
    })
}

fn status_example(status: &str, message: &str) -> Value {
    json!({
        "osl_version": "1.0.0",
        "status": status,
        "message": message
    })
}

fn schema_name<T: ToSchema>() -> String {
    T::name().into_owned()
}

fn auth_db_errors() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("403", "Forbidden", err_example("Unauthorized access.")),
        (
            "500",
            "Internal Server Error",
            err_example("Database error."),
        ),
    ]
}

fn secret_lookup_errors() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("403", "Forbidden", err_example("Unauthorized access.")),
        ("404", "Not Found", err_example("Secret not found.")),
        (
            "500",
            "Internal Server Error",
            err_example("Database error."),
        ),
    ]
}

fn secret_backend_errors() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("400", "Bad Request", err_example("Plugin error.")),
        ("403", "Forbidden", err_example("Unauthorized access.")),
        ("404", "Not Found", err_example("Secret not found.")),
        (
            "500",
            "Internal Server Error",
            err_example("Database error."),
        ),
        (
            "501",
            "Not Implemented",
            err_example("Unsupported backend."),
        ),
    ]
}

fn principal_admin_errors() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("400", "Bad Request", err_example("Principal not found.")),
        ("403", "Forbidden", err_example("Unauthorized access.")),
        (
            "500",
            "Internal Server Error",
            err_example("Database error."),
        ),
    ]
}

fn auth_method_errors() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("403", "Forbidden", err_example("Unauthorized access.")),
        (
            "500",
            "Internal Server Error",
            err_example("Internal error."),
        ),
        (
            "501",
            "Not Implemented",
            err_example("Unsupported authorization"),
        ),
    ]
}

fn build_osl_spec() -> utoipa::openapi::OpenApi {
    let get_secret = schema_name::<ApiResponse<ApiGetSecretResponse>>();
    let put_secret = schema_name::<ApiResponse<ApiPutSecretResponse>>();
    let list_secret = schema_name::<ApiResponse<ApiListSecretResponse>>();
    let destroy_secret = schema_name::<ApiResponse<ApiDestroySecretResponse>>();
    let delete_secret = schema_name::<ApiResponse<ApiDeleteSecretResponse>>();
    let restore_secret = schema_name::<ApiResponse<ApiRestoreSecretResponse>>();
    let taint_secret = schema_name::<ApiResponse<ApiTaintSecretResponse>>();
    let is_tainted_secret = schema_name::<ApiResponse<ApiIsTaintedSecretResponse>>();
    let describe_secret = schema_name::<ApiResponse<ApiDescribeSecretResponse>>();
    let capabilities = schema_name::<ApiResponse<ApiCapabilitiesResponse>>();
    let list_apps = schema_name::<ApiResponse<ApiListAppsResponse>>();
    let list_backends = schema_name::<ApiResponse<ApiListBackendsResponse>>();

    let paths = PathsBuilder::new()
        .path(
            "/secrets/put",
            post_ex(
                &schema_name::<PutSecretRequestData>(),
                Some(json!({
                    "app_id": "my-app",
                    "secret_name": "db-credentials",
                    "store": {
                        "backend_ref": "openbao-rs",
                        "store_path": "secret/data/db"
                    },
                    "data": {
                        "username": "db_user",
                        "password": "super_secret_password"
                    },
                    "options": {
                        "create_only": false
                    },
                    "idempotency_token": null
                })),
                &put_secret,
                "secrets",
                Some(ok_example(json!({
                    "secret_name": "db-credentials",
                    "provider_version_id": "12"
                }))),
                vec![
                    ("400", "Bad Request", err_example("Secret already exists.")),
                    ("403", "Forbidden", err_example("Unauthorized access.")),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Internal error."),
                    ),
                    (
                        "501",
                        "Not Implemented",
                        err_example("Unsupported backend."),
                    ),
                ],
            ),
        )
        .path(
            "/secrets/destroy",
            post(
                &schema_name::<DestroySecretRequestData>(),
                &destroy_secret,
                "secrets",
                Some(ok_example(json!({ "destroyed": true }))),
                secret_backend_errors(),
            ),
        )
        .path(
            "/secrets/delete",
            post(
                &schema_name::<DeleteSecretRequestData>(),
                &delete_secret,
                "secrets",
                Some(ok_example(json!({ "soft_deleted": true }))),
                secret_backend_errors(),
            ),
        )
        .path(
            "/secrets/restore",
            post(
                &schema_name::<RestoreSecretRequestData>(),
                &restore_secret,
                "secrets",
                Some(ok_example(json!({ "restored": true }))),
                secret_backend_errors(),
            ),
        )
        .path(
            "/secrets/taint",
            post(
                &schema_name::<TaintSecretRequestData>(),
                &taint_secret,
                "secrets",
                Some(ok_example(json!({ "tainted": true }))),
                secret_lookup_errors(),
            ),
        )
        .path(
            "/secrets/untaint",
            post(
                &schema_name::<UntaintSecretRequestData>(),
                &taint_secret,
                "secrets",
                Some(ok_example(json!({ "tainted": false }))),
                secret_lookup_errors(),
            ),
        )
        .path(
            "/secrets/get",
            post(
                &schema_name::<GetSecretRequestData>(),
                &get_secret,
                "secrets",
                Some(ok_example(json!({
                    "password": "super_secret_password-new",
                    "username": "db_user-new",
                    "metadata": {
                        "resolved_backend_ref": "openbao-rs",
                        "provider_version_id": "11"
                    }
                }))),
                vec![
                    ("400", "Bad Request", err_example("Plugin error.")),
                    ("403", "Forbidden", err_example("Unauthorized access.")),
                    ("404", "Not Found", err_example("Secret not found.")),
                    (
                        "409",
                        "Conflict",
                        err_example("The secret is tainted and inaccessible."),
                    ),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Database error."),
                    ),
                    (
                        "501",
                        "Not Implemented",
                        err_example("Unsupported backend."),
                    ),
                ],
            ),
        )
        .path(
            "/secrets/list",
            post(
                &schema_name::<ListSecretRequestData>(),
                &list_secret,
                "secrets",
                Some(ok_example(json!([
                    {
                        "secret_name": "db-credentials",
                        "backend": "openbao-rs",
                        "mount_path": "secret/data/db",
                        "tainted": false
                    }
                ]))),
                auth_db_errors(),
            ),
        )
        .path(
            "/secrets/is-tainted",
            post(
                &schema_name::<IsTaintedSecretRequestData>(),
                &is_tainted_secret,
                "secrets",
                Some(ok_example(json!({ "is_tainted": false }))),
                secret_lookup_errors(),
            ),
        )
        .path(
            "/secrets/describe",
            post(
                &schema_name::<DescribeSecretRequestData>(),
                &describe_secret,
                "secrets",
                None,
                secret_backend_errors(),
            ),
        )
        .path(
            "/capabilities/get",
            get(&capabilities, "capabilities", None, auth_db_errors()),
        )
        .path(
            "/apps/list",
            get_with_query(
                vec![(
                    "after_app_id",
                    false,
                    "Return apps with an app_id ordered after this value.",
                )],
                &list_apps,
                "apps",
                Some(ok_example(json!([{ "app_id": "my-app" }]))),
                auth_db_errors(),
            ),
        )
        .path(
            "/backends/list",
            get(
                &list_backends,
                "backends",
                Some(ok_example(json!(["openbao-rs", "aws-secrets-manager"]))),
                vec![("403", "Forbidden", err_example("Unauthorized access."))],
            ),
        )
        .build();

    let components = ComponentsBuilder::new()
        .schema_from::<PutSecretRequestData>()
        .schema_from::<Store>()
        .schema_from::<Options>()
        .schema_from::<GetSecretRequestData>()
        .schema_from::<DeleteSecretRequestData>()
        .schema_from::<DestroySecretRequestData>()
        .schema_from::<RestoreSecretRequestData>()
        .schema_from::<ListSecretRequestData>()
        .schema_from::<TaintSecretRequestData>()
        .schema_from::<UntaintSecretRequestData>()
        .schema_from::<IsTaintedSecretRequestData>()
        .schema_from::<DescribeSecretRequestData>()
        .schema_from::<ApiResponse<ApiGetSecretResponse>>()
        .schema_from::<ApiResponse<ApiPutSecretResponse>>()
        .schema_from::<ApiResponse<ApiListSecretResponse>>()
        .schema_from::<ApiResponse<ApiDestroySecretResponse>>()
        .schema_from::<ApiResponse<ApiDeleteSecretResponse>>()
        .schema_from::<ApiResponse<ApiRestoreSecretResponse>>()
        .schema_from::<ApiResponse<ApiTaintSecretResponse>>()
        .schema_from::<ApiResponse<ApiIsTaintedSecretResponse>>()
        .schema_from::<ApiResponse<ApiDescribeSecretResponse>>()
        .schema_from::<ApiResponse<ApiCapabilitiesResponse>>()
        .schema_from::<ApiResponse<ApiListAppsResponse>>()
        .schema_from::<ApiResponse<ApiListBackendsResponse>>()
        .schema_from::<ApiGetSecretResponse>()
        .schema_from::<ApiGetSecretMetadataResponse>()
        .schema_from::<ApiPutSecretResponse>()
        .schema_from::<ApiListSecretResponse>()
        .schema_from::<ApiDestroySecretResponse>()
        .schema_from::<ApiDeleteSecretResponse>()
        .schema_from::<ApiRestoreSecretResponse>()
        .schema_from::<ApiTaintSecretResponse>()
        .schema_from::<ApiIsTaintedSecretResponse>()
        .schema_from::<ApiDescribeSecretResponse>()
        .schema_from::<ApiCapabilitiesResponse>()
        .schema_from::<ApiListAppsResponse>()
        .schema_from::<ApiListBackendsResponse>()
        .schema_from::<DescribeOutput>()
        .schema_from::<Capability>()
        .schema_from::<BackendCapabilities>()
        .schema_from::<ApiErrorBody>()
        .schema_from::<SuccessStatus>()
        .build();

    OpenApiBuilder::new()
        .paths(paths)
        .components(Some(components))
        .build()
}

fn build_system_spec() -> utoipa::openapi::OpenApi {
    let init = schema_name::<ApiResponse<ApiInitSystemResponse>>();
    let empty = schema_name::<ApiResponse<()>>();

    let paths = PathsBuilder::new()
        .path(
            "/init",
            post_ex(
                &schema_name::<InitSystemData>(),
                Some(json!({
                    "total_shares": 5,
                    "threshold": 2,
                    "plugins_credentials": {
                        "vaultRef": {
                            "token": "s.1a2b3c4d5e6f7g8h",
                            "address": "https://vault.example.com:8200"
                        }
                    }
                })),
                &init,
                "system",
                Some(json!({
                    "osl_version": "1.0.0",
                    "status": "system-initialized",
                    "message": "System initialized",
                    "data": {
                        "shards": [
                            "c2hhcmQtb25l",
                            "c2hhcmQtdHdv",
                            "c2hhcmQtdGhyZWU="
                        ],
                        "root_token": "pk_aBcD1234aBcD1234aBcD1234aBcD1234"
                    }
                })),
                vec![
                    (
                        "400",
                        "Bad Request",
                        err_example("System already initialized."),
                    ),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Database error."),
                    ),
                ],
            ),
        )
        .path(
            "/plugin/config",
            post_ex(
                &schema_name::<PluginConfigData>(),
                Some(json!({
                    "plugins_credentials": {
                        "vaultRef": {
                            "token": "s.1a2b3c4d5e6f7g8h",
                            "address": "https://vault.example.com:8200"
                        }
                    }
                })),
                &empty,
                "system",
                Some(status_example("operation-completed", "Operation completed")),
                vec![
                    ("403", "Forbidden", err_example("System locked.")),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Internal error."),
                    ),
                ],
            ),
        )
        .path(
            "/unlock",
            post(
                &schema_name::<UnlockData>(),
                &empty,
                "system",
                Some(status_example("system-unlocked", "System unlocked")),
                vec![
                    ("403", "Forbidden", err_example("Invalid shamir keys.")),
                    (
                        "503",
                        "Service Unavailable",
                        err_example("System not initialized."),
                    ),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Internal error."),
                    ),
                ],
            ),
        )
        .path(
            "/status",
            get(
                &empty,
                "system",
                Some(status_example("system-unlocked", "System unlocked")),
                vec![],
            ),
        )
        .build();

    let components = ComponentsBuilder::new()
        .schema_from::<InitSystemData>()
        .schema_from::<UnlockData>()
        .schema_from::<PluginConfigData>()
        .schema_from::<ApiResponse<ApiInitSystemResponse>>()
        .schema_from::<ApiInitSystemResponse>()
        .schema_from::<ApiResponse<()>>()
        .schema_from::<ApiErrorBody>()
        .schema_from::<SuccessStatus>()
        .build();

    OpenApiBuilder::new()
        .paths(paths)
        .components(Some(components))
        .build()
}

fn build_app_spec() -> utoipa::openapi::OpenApi {
    let create_app = schema_name::<ApiResponse<ApiCreateAppResponse>>();
    let create_user = schema_name::<ApiResponse<ApiCreateAppUserResponse>>();
    let auth_user = schema_name::<ApiResponse<ApiAuthAppUserResponse>>();
    let auth_challenge = schema_name::<ApiResponse<ApiAuthChallengeResponse>>();
    let grant = schema_name::<ApiResponse<ApiCreateAppGrantResponse>>();
    let revoke = schema_name::<ApiResponse<ApiDeleteAppGrantResponse>>();
    let list_apps = schema_name::<ApiResponse<ApiListAppsResponse>>();
    let empty = schema_name::<ApiResponse<()>>();

    let paths = PathsBuilder::new()
        .path(
            "/create",
            post(
                &schema_name::<CreateAppData>(),
                &create_app,
                "apps",
                Some(ok_example(json!({
                    "app_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f00",
                    "app_name": "billing-service"
                }))),
                vec![
                    ("403", "Forbidden", err_example("Unauthorized access.")),
                    (
                        "409",
                        "Conflict",
                        err_example("Application with the same name already exists."),
                    ),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Database error."),
                    ),
                ],
            ),
        )
        .path(
            "/user/create",
            post_ex(
                &schema_name::<CreateAppUserData>(),
                Some(json!({
                    "name": "ci-runner",
                    "kind": "machine",
                    "credential_kind": "tpm",
                    "data": {
                        "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIB...\n-----END PUBLIC KEY-----",
                        "require_ek_cert": false,
                        "ak_public_tpm2b_b64": "AAEACwAD..."
                    },
                    "expires_at": null,
                    "apps": ["billing-service"]
                })),
                &create_user,
                "app-users",
                Some(ok_example(json!({
                    "principal_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f01",
                    "credential_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f02"
                }))),
                auth_method_errors(),
            ),
        )
        .path(
            "/user/auth",
            post_ex(
                &schema_name::<AuthUserData>(),
                Some(json!({
                    "credential_kind": "tpm",
                    "proof": {
                        "challenge_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f03",
                        "quote": "AAEACw...",
                        "signature": "MEUCIQ..."
                    }
                })),
                &auth_user,
                "app-users",
                Some(ok_example(json!({
                    "token": "tok_aBcD1234aBcD1234aBcD1234aBcD1234",
                    "expires_at": 1_750_000_000
                }))),
                vec![
                    ("400", "Bad Request", err_example("Principal not found.")),
                    ("403", "Forbidden", err_example("Unauthorized access.")),
                    (
                        "500",
                        "Internal Server Error",
                        err_example("Database error."),
                    ),
                    (
                        "501",
                        "Not Implemented",
                        err_example("Unsupported authorization"),
                    ),
                ],
            ),
        )
        .path(
            "/user/auth/challenge",
            post_ex(
                &schema_name::<AuthChallengeData>(),
                Some(json!({
                    "credential_kind": "tpm",
                    "hint": {
                        "ek_pubkey_pem": "-----BEGIN PUBLIC KEY-----\nMIIB...\n-----END PUBLIC KEY-----"
                    }
                })),
                &auth_challenge,
                "app-users",
                Some(ok_example(json!({
                    "challenge_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f03",
                    "expires_at": 1_750_000_000
                }))),
                auth_method_errors(),
            ),
        )
        .path(
            "/user/auth/activate",
            post_ex(
                &schema_name::<AuthActivationData>(),
                Some(json!({
                    "credential_kind": "tpm",
                    "principal_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f01",
                    "credential_id": "018f3c2a-1b2c-7d3e-9f4a-5b6c7d8e9f02",
                    "proof": {
                        "decrypted_secret": "AAEACw..."
                    }
                })),
                &empty,
                "app-users",
                Some(status_example("operation-completed", "Operation completed")),
                auth_method_errors(),
            ),
        )
        .path(
            "/user/grant",
            post(
                &schema_name::<AppGrantData>(),
                &grant,
                "app-grants",
                Some(status_example("operation-completed", "Operation completed")),
                principal_admin_errors(),
            ),
        )
        .path(
            "/user/revoke",
            post(
                &schema_name::<RevokeAppData>(),
                &revoke,
                "app-grants",
                Some(status_example("operation-completed", "Operation completed")),
                principal_admin_errors(),
            ),
        )
        .path(
            "/user/list",
            post(
                &schema_name::<AppListAppsData>(),
                &list_apps,
                "app-grants",
                Some(ok_example(json!([{ "app_id": "billing-service" }]))),
                auth_db_errors(),
            ),
        )
        .build();

    let components = ComponentsBuilder::new()
        .schema_from::<CreateAppData>()
        .schema_from::<CreateAppUserData>()
        .schema_from::<AuthUserData>()
        .schema_from::<AuthChallengeData>()
        .schema_from::<AuthActivationData>()
        .schema_from::<AppGrantData>()
        .schema_from::<RevokeAppData>()
        .schema_from::<AppListAppsData>()
        .schema_from::<PrincipalKind>()
        .schema_from::<PrincipalCredentialKind>()
        .schema_from::<ApiResponse<ApiCreateAppResponse>>()
        .schema_from::<ApiResponse<ApiCreateAppUserResponse>>()
        .schema_from::<ApiResponse<ApiAuthAppUserResponse>>()
        .schema_from::<ApiResponse<ApiAuthChallengeResponse>>()
        .schema_from::<ApiResponse<ApiCreateAppGrantResponse>>()
        .schema_from::<ApiResponse<ApiDeleteAppGrantResponse>>()
        .schema_from::<ApiResponse<ApiListAppsResponse>>()
        .schema_from::<ApiResponse<()>>()
        .schema_from::<ApiCreateAppResponse>()
        .schema_from::<ApiCreateAppUserResponse>()
        .schema_from::<ApiAuthAppUserResponse>()
        .schema_from::<ApiAuthChallengeResponse>()
        .schema_from::<ApiCreateAppGrantResponse>()
        .schema_from::<ApiDeleteAppGrantResponse>()
        .schema_from::<ApiListAppsResponse>()
        .schema_from::<ApiErrorBody>()
        .schema_from::<SuccessStatus>()
        .build();

    OpenApiBuilder::new()
        .paths(paths)
        .components(Some(components))
        .build()
}

pub fn build_spec() -> utoipa::openapi::OpenApi {
    let mut root = OpenApiBuilder::new()
        .info(InfoBuilder::new().title("OSL API").version("1.0.0").build())
        .build();

    root = root.nest("/osl/v1", build_osl_spec());
    root = root.nest("/system", build_system_spec());
    root = root.nest("/app", build_app_spec());
    root
}
