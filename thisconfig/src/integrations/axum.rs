use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};

use serde::Serialize;

pub struct ExtractConfig<T>(pub T);

impl<S, T> FromRequestParts<S> for ExtractConfig<T>
where
    T: ConfigItem,
    S: Send + Sync,
{
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let Some(config) = parts.extensions.get::<Config>() else {
            tracing::error!("Configuration extension not found in request parts");
            return Err(ErrorResponse::internal_server_error());
        };

        let Some(item) = config.get::<T>() else {
            tracing::error!("Configuration item '{}' not found", T::key());
            return Err(ErrorResponse::internal_server_error());
        };

        Ok(ExtractConfig(item))
    }
}

pub struct ExtractOptionalConfig<T>(pub Option<T>);

impl<S, T> FromRequestParts<S> for ExtractOptionalConfig<T>
where
    T: ConfigItem,
    S: Send + Sync,
{
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let Some(config) = parts.extensions.get::<Config>() else {
            tracing::error!("Configuration extension not found in request parts");
            return Err(ErrorResponse::internal_server_error());
        };

        let item = config.get::<T>();

        Ok(ExtractOptionalConfig(item))
    }
}

#[cfg(feature = "validation")]
pub struct ExtractValidatedConfig<T>(pub T);

#[cfg(feature = "validation")]
use validator::Validate;

use crate::{Config, ConfigItem};

#[cfg(feature = "validation")]
impl<S, T> FromRequestParts<S> for ExtractValidatedConfig<T>
where
    T: ConfigItem + Validate,
    S: Send + Sync,
{
    type Rejection = ErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let Some(config) = parts.extensions.get::<Config>() else {
            tracing::error!("Configuration extension not found in request parts");
            return Err(ErrorResponse::internal_server_error());
        };

        let item = config.get_validated::<T>().map_err(|e| {
            tracing::error!("Configuration validation failed for '{}': {e}", T::key());
            ErrorResponse::internal_server_error()
        })?;

        Ok(ExtractValidatedConfig(item))
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    code: u16,
    success: bool,
    message: String,
}

impl ErrorResponse {
    pub fn internal_server_error() -> Self {
        ErrorResponse {
            code: 500,
            success: false,
            message: "Internal Server Error".to_string(),
        }
    }

    pub fn bad_request() -> Self {
        ErrorResponse {
            code: 400,
            success: false,
            message: "Bad Request".to_string(),
        }
    }
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorResponse;
    use axum::{http::StatusCode, response::IntoResponse};

    #[test]
    fn test_into_response_status() {
        let error = ErrorResponse::internal_server_error();
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Deserialize)]
    struct MockConfig {
        value: String,
    }

    impl ConfigItem for MockConfig {
        fn key() -> &'static str {
            "mock"
        }
    }

    #[test]
    fn test_config_wrapper() {
        let mock = MockConfig {
            value: "test".to_string(),
        };

        let config = ExtractConfig(Some(mock.clone()));
        assert_eq!(config.0.as_ref().unwrap().value, "test");
    }

    #[cfg(feature = "validation")]
    #[test]
    fn test_validated_config_wrapper() {
        use validator::Validate;

        #[derive(Debug, Clone, Deserialize, Validate)]
        struct ValidatedMockConfig {
            #[validate(length(min = 1))]
            value: String,
        }

        impl ConfigItem for ValidatedMockConfig {
            fn key() -> &'static str {
                "validated_mock"
            }
        }

        let mock = ValidatedMockConfig {
            value: "test".to_string(),
        };

        let config = ExtractValidatedConfig(mock.clone());
        assert_eq!(config.0.value, "test");
    }
}
