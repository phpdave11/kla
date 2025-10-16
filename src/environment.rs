use std::{
    borrow::Cow,
    fmt::{Display, Write},
    path::PathBuf,
};

use clap::{command, Command};
use config::{builder::DefaultState, Config, ConfigBuilder, File};
use serde::Deserialize;
use skim::SkimItem;

use crate::{Error, Result};

#[derive(Debug)]
pub enum Environment {
    Endpoint(Endpoint),
    Empty,
}

impl Default for Environment {
    fn default() -> Self {
        return Environment::Empty;
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Endpoint(endpoint) => endpoint.fmt(f),
            Self::Empty => Ok(()),
        }
    }
}

impl Environment {
    pub fn new(env: Option<&String>, config: &Config) -> Result<Environment> {
        if let Some(env) = env {
            Ok(Environment::Endpoint(Endpoint::new(env.clone(), config)?))
        } else {
            Ok(Environment::Empty)
        }
    }

    pub fn create_url(&self, uri: &str) -> String {
        match self {
            Environment::Endpoint(endpoint) => endpoint.create_url(uri),
            Environment::Empty => String::from(uri),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Endpoint {
    #[serde(skip)]
    pub name: String,

    #[serde(rename = "url")]
    prefix: String,

    #[serde(rename = "short_description")]
    short_description: Option<String>,

    #[serde(rename = "long_description")]
    long_description: Option<String>,

    #[serde(rename = "template_dir")]
    template_dir: Option<String>,
}

impl Endpoint {
    pub fn new<S>(env: S, config: &Config) -> Result<Endpoint>
    where
        S: Into<String>,
    {
        let env: String = env.into();
        let mut endpoint =
            config.get::<Endpoint>(format!("environment.{}", env.as_str()).as_ref())?;

        // set the name
        endpoint.name = env;

        // normalize the prefix
        if !endpoint.prefix.ends_with("/") {
            endpoint.prefix.push_str("/");
        };

        Ok(endpoint)
    }

    pub fn create_url(&self, uri: &str) -> String {
        // if the uri starts with http or https scheme we assume the uri is
        // a url
        if uri.starts_with("http://") || uri.starts_with("https://") {
            return String::from(uri);
        }

        let mut url = self.prefix.clone();
        url.push_str(uri.trim_start_matches("/"));
        url
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: [{}]\n", self.name, self.prefix)?;

        if let Some(short_description) = self.short_description.as_ref() {
            write!(f, "\tdescription: {}\n", short_description)?;
        }

        Ok(())
    }
}

impl SkimItem for Endpoint {
    fn text(&self) -> Cow<'_, str> {
        Cow::from(&self.name)
    }

    fn preview(&self, _context: skim::PreviewContext) -> skim::ItemPreview {
        let mut s = String::new();
        write!(s, "{}: [{}]\n", &self.name, &self.prefix).expect("writing to string");

        if let Some(long_description) = self.long_description.as_ref() {
            write!(s, "\n{long_description}").expect("writing to string");
        } else if let Some(short_description) = self.short_description.as_ref() {
            write!(s, "\n{short_description}").expect("writing to string");
        }

        skim::ItemPreview::Text(s)
    }
}

pub trait FromEnvironment {
    type Output;

    fn add_source_environment(self, env: &crate::Environment, tmpl: &str) -> Result<Self::Output>;
}

impl FromEnvironment for ConfigBuilder<DefaultState> {
    type Output = Self;

    fn add_source_environment(self, env: &Environment, tmpl: &str) -> Result<Self::Output> {
        let environment = match env {
            Environment::Empty => return Err(Error::KlaError(String::from("no environment set"))),
            Environment::Endpoint(endpoint) => endpoint,
        };

        let template_dir = match environment.template_dir.as_ref() {
            Some(val) => val,
            None => return Err(Error::KlaError(String::from("no template directory set"))),
        };

        let mut template = PathBuf::from(template_dir);
        template.push(tmpl);

        Ok(self.add_source(File::with_name(
            template.as_path().to_str().expect("bad path"),
        )))
    }
}
