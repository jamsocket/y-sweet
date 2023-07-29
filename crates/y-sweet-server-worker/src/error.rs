use serde::Serialize;
use thiserror::Error;
use worker::Response;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Expected server auth header.")]
    ExpectedAuthHeader,
    #[error("Server auth header provided but not valid.")]
    BadAuthHeader,
    #[error("Expected client auth header.")]
    ExpectedClientAuthHeader,
    #[error("Client auth header provided but not valid.")]
    BadClientAuthHeader,
    #[error("Configuration error.")]
    ConfigurationError,
    #[error("Missing 'host' header.")]
    MissingHostHeader,
    #[error("No such document.")]
    NoSuchDocument,
    #[error("Upstream connection error.")]
    UpstreamConnectionError,
}

impl Error {
    fn status_code(&self) -> u16 {
        match self {
            Self::ExpectedAuthHeader => 401,
            Self::BadAuthHeader => 403,
            Self::ConfigurationError => 500,
            Self::MissingHostHeader => 400,
            Self::NoSuchDocument => 404,
            Self::UpstreamConnectionError => 500,
            Self::ExpectedClientAuthHeader => 401,
            Self::BadClientAuthHeader => 403,
        }
    }
}

impl From<Error> for Result<Response, worker::Error> {
    fn from(err: Error) -> Self {
        let status_code = err.status_code();
        let body = format!("{{\"error\": \"{}\"}}", err);
        let mut response = Response::from_json(&body)?;
        response = response.with_status(status_code);
        Ok(response)
    }
}

pub trait IntoResponse<T: Serialize> {
    fn into_response(self) -> Result<Response, worker::Error>;
}

impl<T: Serialize> IntoResponse<T> for Result<T, Error> {
    fn into_response(self) -> Result<Response, worker::Error> {
        match self {
            Ok(response) => Response::from_json(&response),
            Err(err) => err.into(),
        }
    }
}
