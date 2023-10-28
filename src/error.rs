use std::{borrow::Cow, collections::HashMap};

use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use tracing::error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("request path not found")]
    NotFound,

    #[error("error in the request body")]
    UnprocessableEntity {
        errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
    },

    #[error("an internal server error occurred")]
    Eyre(#[from] eyre::Report),
}

impl Error {
    pub fn unprocessable_entity<K, V>(errors: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<Cow<'static, str>>,
        V: Into<Cow<'static, str>>,
    {
        let mut error_map = HashMap::new();
        for (key, val) in errors {
            error_map
                .entry(key.into())
                .or_insert_with(Vec::new)
                .push(val.into());
        }
        Self::UnprocessableEntity { errors: error_map }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Error::NotFound => StatusCode::NOT_FOUND,
            Error::UnprocessableEntity { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Error::Eyre(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::UnprocessableEntity { errors } => {
                #[derive(serde::Serialize)]
                struct Errors {
                    errors: HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>,
                }
                return (StatusCode::UNPROCESSABLE_ENTITY, Json(Errors { errors })).into_response();
            },
            Error::Eyre(ref e) => {
                error!("generic error: {:?}", e);
            },
            _ => (),
        }

        (self.status_code(), self.to_string()).into_response()
    }
}
