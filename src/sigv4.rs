use std::{fmt::Display, time::SystemTime};

use chrono::{DateTime, Utc};
use http::{header::ToStrError, HeaderMap, HeaderValue};
use reqwest::Request;
use url::form_urlencoded::Parse;

use crate::{impl_opt, Opt};

#[derive(thiserror::Error, Debug)]
/// SigningError will be returned from the builder when any issues arise
/// building the signature
pub enum SigningError {
    #[error("Could not build signature: {0}")]
    // BuildError is a general error, usually output because the caller
    // forgot some required field
    BuildError(String),

    #[error("could not build headers: {0}")]
    ToStrError(#[from] ToStrError),
}

#[derive(Debug, Clone)]
/// Signed headers is a list of headers expected in the request we are
/// about to sign. `x-amz-date` will be specified for you based on the
/// date given to the builder (or date will be set to SystemTime::now
/// when left unspecified)
///
/// There are no values in this list, it is just the names of the headers
/// we expect. They should be added to this list the same way we see them
/// in the request. (amazon requires the signature to have them lowercase
/// we handle that for you.)
pub struct SignedHeaders(Vec<String>);

impl SignedHeaders {
    /// push will add an aditional header to the list
    fn push<S: Into<String>>(&mut self, value: S) {
        self.0.push(value.into());
    }
}

/// Create the default value with `host` and `x-amz-date` already there, since they are
/// required when creating a signature
impl Default for SignedHeaders {
    fn default() -> Self {
        Self(vec![String::from("host"), String::from("x-amz-date")])
    }
}

/// This implementation of display will turn the headers into a lowercase
/// ; delimited string, which is what we need in our Authorization.
impl Display for SignedHeaders {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let headers: Vec<String> = self.0.iter().map(|s| s.to_lowercase()).collect();
        write!(f, "{}", headers.join(";"))
    }
}

/// Implementing from allows you to call `SigningError::From("some string")`
/// how nice
impl From<&str> for SigningError {
    fn from(value: &str) -> Self {
        SigningError::BuildError(value.into())
    }
}

#[derive(Debug, Clone)]
pub struct AWSCredentials {
    secret: String,
    key: String,
}

#[derive(Debug, Clone, Default)]
pub struct SigV4Builder {
    /// headers are the headers we want to sign
    headers: SignedHeaders,
    /// The date for the signature, if left unset it will default to
    /// right now
    date: Option<DateTime<Utc>>,
    /// region is the region where the request is being made
    region: Option<String>,
    /// service is the service we intend on making the request against
    service: Option<String>,
    /// credentials hold the AWS credentials for the builder
    credentials: Option<AWSCredentials>,
}

impl SigV4Builder {
    const ALGORITHM: &'static str = "AWS4-HMAC-SHA256";

    /// New Creates a new empty builder
    pub fn new() -> Self {
        Self::default()
    }

    /// header adds a new header to include when signing the request
    pub fn header<S: Into<String>>(mut self, header: S) -> Self {
        self.headers.push(header.into());
        self
    }

    /// date sets the date for the signature, if not set the default
    pub fn date(mut self, date: SystemTime) -> Self {
        self.date = Some(date.into());
        self
    }

    /// region sets the region for the signature
    pub fn region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    /// service sets the service for the signature
    pub fn service(mut self, service: String) -> Self {
        self.service = Some(service);
        self
    }

    /// credentials sets the credentials for the signature
    pub fn credentials<C: Into<AWSCredentials>>(mut self, credentials: C) -> Self {
        self.credentials = Some(credentials.into());
        self
    }

    pub fn sign(self, req: Request) -> Result<Request, SigningError> {
        let Self {
            headers,
            date,
            region,
            service,
            credentials,
        } = self;
        let mut req = req;

        // unwrap things we need or throw errors
        let credentials = credentials.ok_or(SigningError::from(
            "aws credentials are required when creating sigv4 request",
        ))?;
        let date = date.unwrap_or(SystemTime::now().into());
        let region = region.ok_or(SigningError::from(
            "region is required when creating a sigv4 request",
        ))?;
        let service = service.ok_or(SigningError::from(
            "service is required when creating a sigv4 request",
        ))?;

        // we want to be using this format `YYYYMMDDTHHMMSSZ`
        // This makes sure the request has the x-amz-date applied to the
        // request. If the value was set we override it to make sure the
        // signature works.
        req.headers_mut().insert(
            "x-amz-date",
            HeaderValue::from_str(&format!("{}", date.format("%Y%m%d%H%M%SZ")))
                .expect("how can this be invalid"),
        );

        let _authentication = format!("{} Credential={}/{}/{}/{}/aws4_request, SignedHeaders={}, Signature=fe5f80f77d5fa3beca038a248ff027d0445342fe2855ddc963176630326f1024", Self::ALGORITHM, credentials.key, date.format("%Y%m%d"), region, service, headers);
        todo!()
    }
}

#[derive(Debug, Default, Clone)]
pub struct QueryParameters(Vec<(String, String)>);

impl From<Parse<'_>> for QueryParameters {
    fn from(value: Parse<'_>) -> Self {
        let mut params = QueryParameters::default();

        for (key, val) in value {
            let key: String = key.into_owned();
            let val: String = val.into_owned();

            params.0.push((key, val));
        }

        params
    }
}

#[derive(Debug, Default, Clone)]
pub struct Headers(Vec<(String, String)>);

impl TryFrom<&HeaderMap> for Headers {
    type Error = ToStrError;

    fn try_from(value: &HeaderMap) -> Result<Self, Self::Error> {
        let mut headers = Headers::default();

        for (name, value) in value {
            let name: String = name.as_str().into();
            let value: String = value.to_str()?.into();

            headers.0.push((name, value))
        }

        Ok(headers)
    }
}

#[derive(Debug, Default, Clone)]
pub struct CanonicalRequestBuilder<'a> {
    method: Option<String>,
    path: Option<String>,
    query: QueryParameters,
    headers: Headers,
    payload: Option<&'a [u8]>,
}

impl<'a> CanonicalRequestBuilder<'a> {
    pub fn method<S: Into<String>>(mut self, method: S) -> Self {
        self.method = Some(method.into());
        self
    }

    pub fn path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn payload(mut self, payload: &'a [u8]) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn query<Q: Into<QueryParameters>>(mut self, query: Q) -> Self {
        self.query = query.into();
        self
    }

    pub fn headers<E: Into<SigningError>, Q: TryInto<Headers, Error = E>>(
        mut self,
        headers: Q,
    ) -> Result<Self, SigningError> {
        self.headers = headers.try_into().map_err(E::into)?;
        Ok(self)
    }

    pub fn build() -> String {
        todo!()
    }
}

impl_opt!(CanonicalRequestBuilder<'_>);

impl<'a> TryFrom<&'a Request> for CanonicalRequestBuilder<'a> {
    type Error = SigningError;

    fn try_from(value: &'a Request) -> Result<Self, Self::Error> {
        let builder = Self::default();
        builder
            .method(value.method().as_str())
            .path(value.url().path())
            .with_some(
                value.body().map(|b| b.as_bytes()).flatten(),
                CanonicalRequestBuilder::payload,
            )
            .query(value.url().query_pairs())
            .headers(value.headers())
    }
}
