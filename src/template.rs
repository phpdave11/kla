use http::Version;
use reqwest::Response;
use serde::ser::Serialize;
use std::{fs, pin::Pin};
use tera::{Context, Tera};
use tokio::io::AsyncWriteExt;

use crate::Result;

pub struct ContextBuilder {
    data: Context,
}

impl ContextBuilder {
    pub fn new() -> Self {
        ContextBuilder {
            data: Context::new(),
        }
    }

    pub fn insert<T: Serialize + ?Sized, S: Into<String>>(mut self, key: S, val: &T) -> Self {
        self.data.insert(key.into(), val);
        self
    }

    pub async fn insert_response(mut self, response: Response) -> Result<Self> {
        self.data.insert("resp_status", response.status().as_str());

        let headers = response.headers();
        for (name, value) in headers.iter() {
            self.data
                .insert(&format!("resp_headers_{}", name), &value.to_str()?);
        }

        let version = response.version();
        match version {
            Version::HTTP_09 => self.data.insert("resp_http_version", &"HTTP/0.9"),
            Version::HTTP_10 => self.data.insert("resp_http_version", &"HTTP/1.0"),
            Version::HTTP_11 => self.data.insert("resp_http_version", &"HTTP/1.1"),
            Version::HTTP_2 => self.data.insert("resp_http_version", &"HTTP/2.0"),
            Version::HTTP_3 => self.data.insert("resp_http_version", &"HTTP/3.0"),
            _ => self.data.insert("resp_http_version", &"Unknown"),
        }

        let content = response.text().await?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => match Context::from_value(v) {
                Ok(v) => self.data.extend(v),
                _ => (),
            },
            _ => (),
        }
        self.data.insert("resp_body", &content);
        Ok(self)
    }

    pub fn build(self) -> Context {
        self.data
    }
}

pub struct TemplateBuilder {
    template: Option<Tera>,
    context: Option<Context>,
    output: Pin<Box<dyn tokio::io::AsyncWrite>>,
}

impl TemplateBuilder {
    pub fn new(output: Pin<Box<dyn tokio::io::AsyncWrite>>) -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            context: None,
            output,
        }
    }

    pub async fn new_opt_file(path: Option<&String>) -> Result<TemplateBuilder> {
        if let None = path {
            return Ok(TemplateBuilder::new_stdout());
        }

        Ok(TemplateBuilder::new_file(path.unwrap()).await?)
    }

    pub fn new_stdout() -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            context: None,
            output: Box::pin(tokio::io::stdout()),
        }
    }

    pub async fn new_file(path: &str) -> Result<TemplateBuilder> {
        let file = tokio::fs::File::create(path).await?;
        Ok(TemplateBuilder {
            template: None,
            context: None,
            output: Box::pin(file),
        })
    }

    pub fn new_buffer() -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            context: None,
            output: Box::pin(std::io::Cursor::new(Vec::new())),
        }
    }

    pub fn opt_template(self, template: Option<&String>) -> Result<Self> {
        if let Some(template) = template {
            self.template(template)
        } else {
            Ok(self)
        }
    }

    pub fn template(mut self, template: &str) -> Result<Self> {
        let mut tera = Tera::default();
        let mut chars = template.chars();

        self.template = match chars.next() {
            Some('@') => {
                let content = fs::read_to_string(chars.as_str())?;
                tera.add_raw_template("template", &content)?;
                Some(tera)
            }
            Some(_) => {
                tera.add_raw_template("template", template)?;
                Some(tera)
            }
            None => None,
        };

        Ok(self)
    }

    pub fn context(mut self, context: Context) -> Self {
        self.context = Some(context);
        self
    }

    pub fn build(self) -> Result<Template> {
        Ok(Template {
            template: self.template.unwrap_or_default(),
            output: self.output,
            context: self.context.unwrap_or_else(|| Context::new()),
        })
    }
}

pub struct Template {
    template: Tera,
    output: Pin<Box<dyn tokio::io::AsyncWrite>>,
    context: Context,
}

impl Template {
    pub async fn render(self) -> Result<()> {
        let Template {
            template,
            mut output,
            context,
        } = self;

        let template = template.render("template", &context)?;
        output.as_mut().write_all(template.as_bytes()).await?;

        Ok(())
    }
}
