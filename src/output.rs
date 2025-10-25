use std::pin::Pin;

use crate::{impl_opt, impl_when, ContextBuilder, FetchMany, Result};
use http::Method;
use reqwest::Response;
use tera::Tera;
use tokio::{
    fs::File,
    io::{stdout, AsyncWriteExt},
};

// OutputBuilder collects all the info needed to render the output once
// kla has made the http request. (or reqwest rather)
pub struct OutputBuilder {
    // response is the http response that we got
    response: Response,
    // tmpl holds all the templates
    tmpl: Tera,
    prelude: Vec<String>,

    // output
    prelude_output: Option<Pin<Box<dyn tokio::io::AsyncWrite>>>,
    output: Pin<Box<dyn tokio::io::AsyncWrite>>,
}

impl OutputBuilder {
    // new returns a new output builder. If left unchanged a call to render would
    // output nothing
    pub fn new(resp: Response) -> Self {
        OutputBuilder {
            response: resp,
            output: Box::pin(stdout()),
            prelude_output: None,
            tmpl: Tera::default(),
            prelude: vec![],
        }
    }

    /// opt_output takes a command line argument and turns it into an output.
    /// the value Some(`-`) will output to standard out as will None
    /// any value passed is interpreted as a file path, and a new file
    /// is created.
    /// This output is used as the output location of the main body or template
    /// output of the request. Defaults to standard out
    pub async fn opt_output(mut self, output: Option<&String>) -> Result<Self> {
        self.output = match output.map(|v| v.as_str()) {
            Some("-") => Box::pin(stdout()),
            Some(output) => Box::pin(File::create_new(output).await?),
            None => Box::pin(stdout()),
        };
        Ok(self)
    }

    /// opt_prelude_output takes a command line argument and turns it into an output.
    /// the value Some(`-`) will output to standard out as will None
    /// any value passed is interpreted as a file path, and a new file
    /// is created
    /// This output is used as the location of the prelude (Headers, etc) Defaults to
    /// main body output
    pub async fn opt_prelude_output(mut self, output: Option<&String>) -> Result<Self> {
        self.prelude_output = match output.map(|v| v.as_str()) {
            Some("-") => Some(Box::pin(stdout())),
            Some(output) => Some(Box::pin(File::create_new(output).await?)),
            None => None,
        };
        Ok(self)
    }

    // output sets the output of kla. This defaults to standard out
    pub fn output(mut self, output: Pin<Box<dyn tokio::io::AsyncWrite>>) -> Self {
        self.output = output;
        self
    }

    pub fn prelude_request<S>(mut self, method: &Method, url: &str, body: Option<S>) -> Self
    where
        S: Into<String>,
    {
        self.prelude.push(format!("Request: {}: {}", method, &url));
        if let Some(body) = body {
            self.prelude.push(body.into());
        }

        self
    }

    // header_prelude adds a header to the prelude
    pub fn header_prelude(mut self) -> Self {
        let mut buf = String::from("Response Headers\n");

        for (key, val) in self.response.headers() {
            buf.push_str(format!("\t{}: {:?}\n", key.as_str(), val).as_str());
        }
        self.prelude.push(buf);
        self
    }

    // header_prelude adds a header to the prelude
    pub fn code_prelude(mut self) -> Self {
        self.prelude.push(format!("{}", self.response.status()));
        self
    }

    // header_prelude adds a header to the prelude
    pub fn version_prelude(mut self) -> Self {
        self.prelude
            .push(format!("Version: {:?}", self.response.version()));
        self
    }

    // output sets the output of kla. This defaults to standard out
    pub fn prelude_output(mut self, output: Pin<Box<dyn tokio::io::AsyncWrite>>) -> Self {
        self.prelude_output = Some(output);
        self
    }

    // opt template sets the template
    pub fn opt_template(mut self, template: Option<&String>) -> Result<Self> {
        let template = match template {
            Some(template) => template,
            None => return Ok(self),
        };

        // TODO: Add ability to reference files or standard input
        self.tmpl.add_raw_template("body", template)?;
        Ok(self)
    }

    // build creates the output
    pub async fn render(self) -> Result<()> {
        let OutputBuilder {
            mut response,
            tmpl,
            mut prelude_output,
            mut output,
            prelude,
        } = self;

        let prelude_output = prelude_output.as_mut().unwrap_or(&mut output);
        for item in prelude {
            let lines = item.split("\n");
            for line in lines {
                prelude_output.write_all("> ".as_bytes()).await?;
                prelude_output.write_all(line.as_bytes()).await?;
                prelude_output.write_all("\n".as_bytes()).await?;
            }
        }

        // Write the body output
        match tmpl.has("body") {
            true => {
                let buf = tmpl.render(
                    "body",
                    &ContextBuilder::new()
                        .insert_response(response)
                        .await?
                        .build(),
                )?;
                output.write_all(buf.as_bytes()).await?;
            }
            false => {
                while let Some(chunk) = response.chunk().await? {
                    output.write_all(chunk.as_ref()).await?;
                }
            }
        }

        Ok(())
    }
}

impl_when!(OutputBuilder);
impl_opt!(OutputBuilder);
