use std::pin::Pin;

use crate::{ContextBuilder, Result, Template, TemplateBuilder};
use reqwest::Response;
use tokio::{
    fs::File,
    io::{stdout, AsyncWriteExt},
};

// OutputBuilder collects all the info needed to render the output once
// kla has made the http request. (or reqwest rather)
pub struct OutputBuilder {
    response: Response,
    output: Pin<Box<dyn tokio::io::AsyncWrite>>,
    template: Option<String>,
}

impl OutputBuilder {
    // new returns a new output builder. If left unchanged a call to render would
    // output nothing
    pub fn new(resp: Response) -> Self {
        OutputBuilder {
            response: resp,
            output: Box::pin(stdout()),
            template: None,
        }
    }

    // opt_output takes a command line argument and turns it into an output.
    // the value Some(`-`) will output to standard out as will None
    // any value passed is interpreted as a file path, and a new file
    // is created
    pub async fn opt_output(mut self, output: Option<&String>) -> Result<Self> {
        self.output = match output.map(|v| v.as_str()) {
            Some("-") => Box::pin(stdout()),
            Some(output) => Box::pin(File::create_new(output).await?),
            None => Box::pin(stdout()),
        };
        Ok(self)
    }

    // output sets the output of kla. This defaults to standard out
    pub fn output(mut self, output: Pin<Box<dyn tokio::io::AsyncWrite>>) -> Self {
        self.output = output;
        self
    }

    // opt template sets the template
    pub fn opt_template(mut self, template: Option<&String>) -> Self {
        self.template = template.map(|k| k.clone());
        self
    }

    // build creates the output
    pub async fn build(self) -> Result<Output> {
        let OutputBuilder {
            response,
            output,
            template,
        } = self;

        let output = match template {
            Some(template) => {
                let context = ContextBuilder::new()
                    .insert_response(response)
                    .await?
                    .build();

                let template = TemplateBuilder::new(output)
                    .context(context)
                    .template(template.as_ref())?
                    .build()?;

                Output::Template(template)
            }
            None => Output::Response(response, output),
        };

        Ok(output)
    }
}

// OutputContext holds the actual data. This may be the template context if a template
// is set or the response if we are handling output manually
pub enum Output {
    // Template holds the template and context we want to run
    Template(Template),
    // Response holds onto the actual response so we can process
    // it manually
    Response(reqwest::Response, Pin<Box<dyn tokio::io::AsyncWrite>>),
    // do not output anything
    None,
}

impl Output {
    pub async fn output(self) -> Result<()> {
        match self {
            Self::Template(tmpl) => tmpl.render().await,
            Self::Response(mut resp, mut writer) => {
                while let Some(chunk) = resp.chunk().await? {
                    writer.write_all(chunk.as_ref()).await?;
                }
                Ok(())
            }
            Self::None => Ok(()),
        }
    }
}
