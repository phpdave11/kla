use http::Version;
use reqwest::Response;
use std::fs;
use tera::{Context, Tera};

use crate::error::Error;

pub struct TemplateBuilder {
    template: Option<Tera>,
    failure_template: Option<Tera>,
    response: Option<Response>,
    context: Option<Context>,
    output: Box<dyn std::io::Write>,
}

impl TemplateBuilder {
    pub fn new(output: Box<dyn std::io::Write>) -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            failure_template: None,
            response: None,
            context: None,
            output,
        }
    }

    pub fn new_opt_file(path: Option<&String>) -> Result<TemplateBuilder, Error> {
        if let None = path {
            return Ok(TemplateBuilder::new_stdout());
        }

        Ok(TemplateBuilder::new_file(path.unwrap())?)
    }

    pub fn new_stdout() -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            failure_template: None,
            response: None,
            context: None,
            output: Box::new(std::io::stdout()),
        }
    }

    pub fn new_file(path: &str) -> Result<TemplateBuilder, Error> {
        let file = std::fs::File::create(path)?;
        Ok(TemplateBuilder {
            template: None,
            failure_template: None,
            response: None,
            context: None,
            output: Box::new(file),
        })
    }

    pub fn new_buffer() -> TemplateBuilder {
        TemplateBuilder {
            template: None,
            failure_template: None,
            response: None,
            context: None,
            output: Box::new(std::io::Cursor::new(Vec::new())),
        }
    }

    fn parse_template(template: Option<&String>) -> Result<Tera, Error> {
        let mut tera = Tera::default();
        if let None = template {
            tera.add_raw_template("template", "{{ resp_body }}")?;
            return Ok(tera);
        }
        let template = template.unwrap();

        let mut chars = template.chars();
        match chars.next() {
            Some('@') => {
                let content = fs::read_to_string(chars.as_str())?;
                tera.add_raw_template("template", &content)
            }
            Some(_) => tera.add_raw_template("template", template),
            None => tera.add_raw_template("template", "{{ resp_body }}"),
        }?;

        Ok(tera)
    }

    pub fn opt_template(mut self, template: Option<&String>) -> Result<Self, Error> {
        self.template = Some(Self::parse_template(template)?);
        Ok(self)
    }

    pub fn opt_failure_template(mut self, template: Option<&String>) -> Result<Self, Error> {
        self.failure_template = Some(Self::parse_template(template)?);
        Ok(self)
    }

    pub fn response(mut self, response: Response) -> Self {
        self.response = Some(response);
        self
    }

    pub fn build(self) -> Result<Template, Error> {
        Ok(Template {
            template: self.template,
            failure_template: self.failure_template,
            response: self.response.ok_or(Error::InvalidArguments(
                "you must supply a response".to_owned(),
            ))?,
            output: self.output,
            context: self.context.unwrap_or(Context::new()),
        })
    }
}

pub struct Template {
    template: Option<Tera>,
    failure_template: Option<Tera>,
    output: Box<dyn std::io::Write>,
    response: Response,
    context: Context,
}

// TODO: what is the scope of this shit? This needs to be refactored
impl Template {
    pub async fn send(self) -> Result<(), Error> {
        let Template {
            template,
            failure_template,
            mut output,
            response,
            mut context,
        } = self;

        context.insert("resp_status", response.status().as_str());

        let headers = response.headers();
        for (name, value) in headers.iter() {
            context.insert(&format!("resp_headers_{}", name), &value.to_str()?);
        }

        let version = response.version();
        match version {
            Version::HTTP_09 => context.insert("resp_http_version", &"HTTP/0.9"),
            Version::HTTP_10 => context.insert("resp_http_version", &"HTTP/1.0"),
            Version::HTTP_11 => context.insert("resp_http_version", &"HTTP/1.1"),
            Version::HTTP_2 => context.insert("resp_http_version", &"HTTP/2.0"),
            Version::HTTP_3 => context.insert("resp_http_version", &"HTTP/3.0"),
            _ => context.insert("resp_http_version", &"Unknown"),
        }

        let template = if response.status().is_success() {
            &template
        } else {
            &failure_template
        };

        let content = response.text().await?;
        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(v) => match Context::from_value(v) {
                Ok(v) => context.extend(v),
                _ => (),
            },
            _ => (),
        }
        context.insert("resp_body", &content);

        match template {
            None => output.write_all(content.as_bytes())?,
            Some(template) => template.render_to("template", &context, &mut output)?,
        }

        Ok(())
    }
}
