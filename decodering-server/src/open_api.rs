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
use utoipa::openapi::Response;
use utoipa::openapi::path::OperationBuilder;
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;

use crate::handlers::osl::payload::GetSecretRequestData;
use crate::handlers::osl::response::ApiGetSecretMetadataResponse;
use crate::handlers::osl::response::ApiGetSecretResponse;
use crate::handlers::response::ApiErrorBody;
use crate::handlers::response::ApiResponse;
use crate::handlers::response::SuccessStatus;

fn json_body(schema: &str) -> utoipa::openapi::request_body::RequestBody {
    RequestBodyBuilder::new()
        .content(
            "application/json",
            ContentBuilder::new()
                .schema(Some(RefOr::Ref(Ref::from_schema_name(schema))))
                .build(),
        )
        .required(Some(utoipa::openapi::Required::True))
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
    let mut op = OperationBuilder::new()
        .request_body(Some(json_body(req)))
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

// fn get(desc: &str, resp: &str, tag: &str, example: Option<Value>) -> PathItem {
//     PathItem::new(
//         HttpMethod::Get,
//         OperationBuilder::new()
//             .response("200", json_response(desc, resp, example))
//             .tag(tag)
//             .build(),
//     )
// }

fn build_osl_spec() -> utoipa::openapi::OpenApi {
    let paths = PathsBuilder::new()
        .path(
            "/secrets/get",
            post(
                "GetSecretRequestData",
                &schema_name::<ApiResponse<ApiGetSecretResponse>>(),
                "secrets",
                Some(json!({
                    "osl_version": "1.0.0",
                    "status": "operation-completed",
                    "message": "Operation completed",
                    "data": {
                      "password": "super_secret_password-new",
                      "username": "db_user-new",
                      "metadata": {
                        "resolved_backend_ref": "openbao-rs",
                        "provider_version_id": "11"
                      }
                    }
                })),
                vec![(
                    "403",
                    "Forbidden",
                    json!({
                        "osl_version": "1.0.0",
                        "error": {
                          "code": "operation-failed",
                          "message": "Operation failed.",
                          "detail": "Unauthorized access."
                        }
                    }),
                )],
            ),
        )
        .build();

    let components = ComponentsBuilder::new()
        .schema_from::<GetSecretRequestData>()
        .schema_from::<ApiResponse<ApiGetSecretResponse>>()
        .schema_from::<ApiGetSecretResponse>()
        .schema_from::<ApiGetSecretMetadataResponse>()
        .schema_from::<ApiErrorBody>()
        .schema_from::<SuccessStatus>()
        .build();

    OpenApiBuilder::new()
        .paths(paths)
        .components(Some(components))
        .build()
}

fn schema_name<T: ToSchema>() -> String {
    T::name().into_owned()
}

fn build_app_spec() -> utoipa::openapi::OpenApi {
    let paths = PathsBuilder::new()
        // .path(
        //     "/login",
        //     post("LoginRequest", "Token issued", "LoginResponse", "app"),
        // )
        // .path("/profile", get("Profile", "LoginResponse", "app"))
        .build();

    let components = ComponentsBuilder::new()
        // .schema_from::<LoginRequest>()
        // .schema_from::<LoginResponse>()
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
    root = root.nest("/app", build_app_spec());
    root
}
