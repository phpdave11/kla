use duration_string::DurationString;
use http::Version;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Body, RequestBuilder,
};
use std::str::FromStr;
use std::{
    collections::HashMap,
    fs,
    io::{self, Read},
    time::Duration,
};

use crate::{Error, Result};

// This allows us to extend the reqwest RequestBuilder so that we can pass data from clap
// directly into it, creating a seamless interface. This implementation leaves the raw data
// within clap, and greatly reduces the number of copies needed.
pub trait KlaRequestBuilder {
    // opt_headers takes the headers from the `--header` argument and applies them to the
    // request being created.
    fn opt_headers<'a, T>(self, headers: Option<T>) -> Result<RequestBuilder>
    where
        T: Iterator<Item = &'a String>;

    fn opt_query<'a, T>(self, headers: Option<T>) -> Result<RequestBuilder>
    where
        T: Iterator<Item = &'a String>;

    fn opt_form<'a, T>(self, form: Option<T>) -> Result<RequestBuilder>
    where
        T: Iterator<Item = &'a String>;

    fn opt_body<'a>(self, body: Option<&String>) -> Result<RequestBuilder>;

    fn opt_basic_auth(self, userpass: Option<&String>) -> RequestBuilder;

    fn opt_bearer_auth(self, token: Option<&String>) -> RequestBuilder;

    fn opt_timeout(self, timeout: Option<&String>) -> Result<RequestBuilder>;

    fn opt_version(self, version: Option<&String>) -> Result<RequestBuilder>;
}

impl KlaRequestBuilder for RequestBuilder {
    fn opt_version(self, version: Option<&String>) -> Result<RequestBuilder> {
        if let None = version {
            return Ok(self);
        }

        let version = match version.unwrap().as_str() {
            "0.9" => Ok(Version::HTTP_09),
            "1.0" => Ok(Version::HTTP_10),
            "1.1" => Ok(Version::HTTP_11),
            "2.0" => Ok(Version::HTTP_2),
            "3.0" => Ok(Version::HTTP_3),
            _ => Err(Error::InvalidArguments(String::from(
                "invalid http version",
            ))),
        }?;

        Ok(self.version(version))
    }

    fn opt_timeout(self, timeout: Option<&String>) -> Result<RequestBuilder> {
        if let None = timeout {
            return Ok(self);
        }

        // duration_string?!?!?!?! why do you return a string as an error
        // what the f**k is wrong with you.
        // Also thanks for the library!
        let d: Duration = match DurationString::from_str(timeout.unwrap()) {
            Ok(v) => Ok(v),
            Err(msg) => Err(Error::InvalidArguments(msg)),
        }?
        .into();

        Ok(self.timeout(d))
    }

    fn opt_basic_auth(self, userpass: Option<&String>) -> RequestBuilder {
        if let None = userpass {
            return self;
        }
        let userpass = userpass.unwrap();
        let mut parts = userpass.splitn(2, ":");
        self.basic_auth(parts.next().unwrap(), parts.next())
    }

    fn opt_bearer_auth(self, token: Option<&String>) -> RequestBuilder {
        if let None = token {
            return self;
        }

        self.bearer_auth(token.unwrap())
    }

    fn opt_body<'a>(self, body: Option<&String>) -> Result<RequestBuilder> {
        if let None = body {
            return Ok(self);
        }
        let body = body.unwrap();

        let mut body_chars = body.chars();

        let body = match body_chars.next() {
            Some('@') => {
                let name = body_chars.collect::<String>();
                Some(Body::from(fs::read_to_string(name)?))
            }
            Some('-') => {
                let mut buf = String::new();
                io::stdin().read_to_string(&mut buf)?;
                Some(Body::from(buf))
            }
            Some(_) => Some(Body::from(body.to_owned())),
            None => None,
        }
        .ok_or(Error::InvalidArguments("you must supply a body".to_owned()))?;

        Ok(self.body(body))
    }

    fn opt_query<'a, T>(self, query: Option<T>) -> Result<RequestBuilder>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = query {
            return Ok(self);
        }

        let mut map = HashMap::new();
        query
            .unwrap()
            .map(|q| {
                let mut key_val = q.splitn(2, "=");
                let name = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{q} is not a valid key=value"
                    )))?
                    .trim();
                let value = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{q} is not a valid key=value"
                    )))?
                    .trim();

                map.insert(name, value);

                Ok(())
            })
            .collect::<Result<()>>()?;

        Ok(self.query(&map))
    }

    fn opt_form<'a, T>(self, form: Option<T>) -> Result<RequestBuilder>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = form {
            return Ok(self);
        }

        let mut map = HashMap::new();
        form.unwrap()
            .map(|formval| {
                let mut key_val = formval.splitn(2, "=");
                let name = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{formval} is not a valid key=value"
                    )))?
                    .trim();
                let value = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{formval} is not a valid key=value"
                    )))?
                    .trim();

                map.insert(name, value);

                Ok(())
            })
            .collect::<Result<()>>()?;

        Ok(self.form(&map))
    }

    fn opt_headers<'a, T>(self, headers: Option<T>) -> Result<RequestBuilder>
    where
        T: Iterator<Item = &'a String>,
    {
        if let None = headers {
            return Ok(self);
        }

        let mut map = HeaderMap::new();
        headers
            .unwrap()
            .map(|header| {
                let mut key_val = header.splitn(2, ":");
                let name = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{header} is not a valid http header"
                    )))?
                    .trim();
                let value = key_val
                    .next()
                    .ok_or(Error::InvalidArguments(format!(
                        "{header} is not a valid http header"
                    )))?
                    .trim();

                map.insert(
                    HeaderName::from_bytes(name.as_bytes())?,
                    HeaderValue::from_bytes(value.as_bytes())?,
                );

                Ok(())
            })
            .collect::<Result<()>>()?;

        Ok(self.headers(map))
    }
}
