use http::Version;
use reqwest::Response;
use serde::ser::Serialize;
use std::pin::Pin;
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

pub trait OptRender {
    fn render_some(&self, name: &str, context: &Context) -> tera::Result<Option<String>>;
}

impl OptRender for Tera {
    fn render_some(&self, template_name: &str, context: &Context) -> tera::Result<Option<String>> {
        if self
            .get_template_names()
            .find(|x| *x == template_name)
            .is_some()
        {
            self.render(template_name, context).map(|v| Some(v))
        } else {
            Ok(None)
        }
    }
}

pub trait FetchMany {
    fn has(&self, name: &str) -> bool;
    /// fetch_with_prefix will fetch a RenderGroup of the templates that have the prefixed name.
    fn fetch_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
        context: &Context,
    ) -> impl Iterator<Item = RenderGroup<'a>>;
}

impl FetchMany for Tera {
    fn has<'a>(&self, name: &str) -> bool {
        self.get_template_names()
            .filter(move |tmpl| *tmpl == name)
            .next()
            .is_some()
    }

    /// fetch_with_prefix will fetch a RenderGroup of the templates that have the prefixed name.
    fn fetch_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
        context: &Context,
    ) -> impl Iterator<Item = RenderGroup<'a>> {
        self.get_template_names()
            .filter(move |tmpl| tmpl.starts_with(prefix))
            .map(move |f| RenderGroup {
                name: f.strip_prefix(prefix).unwrap_or(f).into(),
                tmpl_name: f.into(),
                tmpl: self,
                context: context.clone(),
            })
    }
}

/// A RenderGroup has all the context required to render a template held within
/// a Tera object.
pub struct RenderGroup<'a> {
    pub name: String,
    pub tmpl_name: String,
    pub tmpl: &'a Tera,
    pub context: Context,
}

impl<'a> RenderGroup<'a> {
    /// render will output the value of the evaluated template
    pub fn render(&self) -> std::result::Result<String, tera::Error> {
        self.tmpl.render(self.tmpl_name.as_str(), &self.context)
    }

    /// return the name of the template which will be rendered
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}
