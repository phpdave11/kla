use clap::{command, Arg, ArgMatches, Command};
use config::{
    builder::DefaultState, Config, ConfigBuilder, ConfigError, File, FileFormat, FileSourceFile,
    FileStoredFormat, Map, Source, Value,
};
use serde::Deserialize;
use std::fmt::Debug;
use std::path::Path;
use tera::{Context, Tera};

use crate::{
    impl_opt,
    opt::{Ok, Opt},
};

#[derive(Debug)]
pub struct OptionalFile<F: FileStoredFormat + 'static>(Option<File<FileSourceFile, F>>);

impl<F> OptionalFile<F>
where
    F: FileStoredFormat + 'static,
{
    pub fn new(path: &str, format: F) -> OptionalFile<F> {
        if !Path::new(path).exists() {
            return OptionalFile(None);
        }

        OptionalFile(Some(File::new(path, format)))
    }

    pub fn with_name(path: &str) -> OptionalFile<FileFormat> {
        if !Path::new(path).exists() {
            return OptionalFile(None);
        }

        OptionalFile(Some(File::with_name(path)))
    }
}

impl<F> Source for OptionalFile<F>
where
    F: FileStoredFormat + Debug + Clone + Send + Sync + 'static,
{
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        match self.0.as_ref() {
            Some(file) => file.clone_into_box(),
            None => Box::new(OptionalFile::<F>(None)),
        }
    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        match self.0.as_ref() {
            Some(file) => file.collect(),
            None => Ok(Map::new()),
        }
    }

    fn collect_to(&self, cache: &mut Value) -> Result<(), ConfigError> {
        match self.0.as_ref() {
            Some(file) => file.collect_to(cache),
            None => Ok(()),
        }
    }
}

// HeaderConfig defines the values in the config needed to create a header
#[derive(Deserialize)]
pub struct ConfigKV {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "value")]
    pub value: String,
}

#[derive(Deserialize)]
struct ConfigCommand {
    #[serde(skip)]
    pub name: String,

    #[serde(rename = "short_description")]
    short_description: Option<String>,

    #[serde(rename = "description")]
    description: Option<String>,

    #[serde(rename = "arg", default)]
    args: Vec<ConfigArg>,

    #[serde(rename = "body")]
    body: Option<String>,
    #[serde(rename = "uri")]
    uri: Option<String>,
    #[serde(rename = "method", default)]
    method: Option<String>,
    #[serde(rename = "header", default)]
    header: Vec<ConfigKV>,
    #[serde(rename = "query", default)]
    query: Vec<ConfigKV>,
    #[serde(rename = "form", default)]
    form: Vec<ConfigKV>,
}

impl ConfigCommand {
    pub fn with_name<S: Into<String>, C: TryInto<Self, Error = crate::Error>>(
        name: S,
        conf: C,
    ) -> Result<ConfigCommand, crate::Error> {
        let mut cmd: Self = conf.try_into()?;
        cmd.name = name.into();
        Ok(cmd)
    }
}

impl TryFrom<Config> for ConfigCommand {
    type Error = crate::Error;

    fn try_from(value: Config) -> Result<Self, Self::Error> {
        let v = value.try_deserialize()?;
        Ok(v)
    }
}

impl TryFrom<&Config> for ConfigCommand {
    type Error = crate::Error;

    fn try_from(value: &Config) -> Result<Self, Self::Error> {
        let v = value.clone().try_deserialize()?;
        Ok(v)
    }
}

impl TryFrom<ConfigCommand> for Command {
    type Error = crate::Error;

    fn try_from(value: ConfigCommand) -> Result<Self, Self::Error> {
        let command = command!()
            .name(&value.name)
            .with_some(value.short_description.as_ref(), Command::about)
            .with_some(value.description.as_ref(), Command::long_about)
            .with_ok_value(
                value
                    .args
                    .into_iter()
                    .map(|v| ConfigArg::try_into(v))
                    .collect::<Result<Vec<Arg>, Self::Error>>(),
                Command::args,
            )?;

        Ok(command)
    }
}

/// enables a oneliner to turn a config into a Command
pub trait CommandWithName: Sized {
    type Error;

    fn command_with_name(self, name: &str) -> Result<Command, Self::Error>;
}

/// This implementation makes it possible for &Config and Config (both of which
/// implement TryInto<ConfigCommand>) to call the shorthand function below
impl<T: TryInto<ConfigCommand, Error = crate::Error>> CommandWithName for T {
    type Error = crate::Error;

    /// Turn the a Config or &Config object into a Command. This function expects
    /// the config to have the following structure: _optional_
    ///
    /// _short_description_: Short description of the command
    /// _description_: Description of the command
    /// _arg_: Array of Arguments
    fn command_with_name(self, name: &str) -> Result<Command, Self::Error> {
        Command::try_from(ConfigCommand::with_name(name, self)?)
    }
}

pub trait TemplateArgsContext: Sized {
    type Error;
    fn template_args(self, tmpl_conf: &Config, args: &ArgMatches) -> Result<Self, Self::Error>;
}

impl TemplateArgsContext for Context {
    type Error = crate::Error;
    /// template_arguments takes the values provided by command line arguments and
    /// passes them into the context.
    fn template_args(mut self, tmpl_conf: &Config, args: &ArgMatches) -> Result<Self, Self::Error> {
        for arg in tmpl_conf.get_array("arg").unwrap_or_default() {
            let arg_conf: ConfigArg = arg.try_deserialize()?;

            match arg_conf.arg_type() {
                ConfigArgType::List => {
                    let val = match args.get_many::<String>(&arg_conf.name) {
                        Some(val) => val.collect::<Vec<_>>(),
                        None => continue,
                    };

                    self.insert(&arg_conf.name, &val)
                }
                ConfigArgType::Single => {
                    let val = match args.get_one::<String>(&arg_conf.name) {
                        Some(val) => val,
                        None => continue,
                    };

                    self.insert(&arg_conf.name, val)
                }
            }
        }

        Ok(self)
    }
}

pub trait KlaTemplateConfig: Sized {
    type Error;
    fn with_kla_template(self, conf: &Config) -> Result<Self, Self::Error>;
    fn opt_template<S: AsRef<str>>(
        self,
        name: &str,
        content: Option<S>,
    ) -> Result<Self, Self::Error>;
    fn template(self, name: &str, content: &str) -> Result<Self, Self::Error>;
}

impl KlaTemplateConfig for Tera {
    type Error = crate::Error;

    fn with_kla_template(self, conf: &Config) -> Result<Self, Self::Error> {
        let config: ConfigCommand = conf.clone().try_deserialize()?;
        let mut context = self
            .opt_template("body", config.body)?
            .template(
                "uri",
                config.uri.unwrap_or_else(|| String::from("/")).as_str(),
            )?
            .template(
                "method",
                config
                    .method
                    .unwrap_or_else(|| String::from("GET"))
                    .as_str(),
            )?;

        for header in &config.header {
            context = context.template(&format!("header.{}", header.name), &header.value)?;
        }

        for header in &config.query {
            context = context.template(&format!("query.{}", header.name), &header.value)?;
        }

        for header in &config.form {
            context = context.template(&format!("form.{}", header.name), &header.value)?;
        }

        Ok(context)
    }

    fn opt_template<'a, S: AsRef<str>>(
        mut self,
        name: &str,
        content: Option<S>,
    ) -> Result<Self, Self::Error> {
        if let Some(body) = content {
            self.add_raw_template(name, body.as_ref())?;
        }
        Ok(self)
    }

    fn template(mut self, name: &str, content: &str) -> Result<Self, Self::Error> {
        self.add_raw_template(name, content.as_ref())?;
        Ok(self)
    }
}

#[derive(Deserialize, Copy, Clone)]
enum ConfigArgType {
    List,
    Single,
}

impl Default for ConfigArgType {
    fn default() -> Self {
        Self::Single
    }
}

#[derive(Deserialize)]
struct ConfigArg {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "type", default)]
    arg_type: ConfigArgType,
    #[serde(rename = "short")]
    short: Option<char>,
    #[serde(rename = "short_aliases", default)]
    short_aliases: Vec<char>,
    #[serde(rename = "long")]
    long: Option<String>,
    #[serde(rename = "aliases", default)]
    aliases: Vec<String>,
    #[serde(rename = "help")]
    help: Option<String>,
    #[serde(rename = "long_help")]
    long_help: Option<String>,
    #[serde(rename = "next_line_help")]
    next_line_help: Option<bool>,
    #[serde(rename = "required")]
    required: Option<bool>,
    #[serde(rename = "trailing_var_arg")]
    trailing_var_arg: Option<bool>,
    #[serde(rename = "last")]
    last: Option<bool>,
    #[serde(rename = "exclusive")]
    exclusive: Option<bool>,
    #[serde(rename = "value_name")]
    value_name: Option<String>,
    #[serde(rename = "allow_hyphen_values")]
    allow_hyphen_values: Option<bool>,
    #[serde(rename = "allow_negative_numbers")]
    allow_negative_numbers: Option<bool>,
    #[serde(rename = "require_equals")]
    require_equals: Option<bool>,
    #[serde(rename = "value_delimiter")]
    value_delimiter: Option<char>,
    #[serde(rename = "value_terminator")]
    value_terminator: Option<String>,
    #[serde(rename = "raw")]
    raw: Option<bool>,
    #[serde(rename = "default_value")]
    default_value: Option<String>,
    #[serde(rename = "default_values", default)]
    default_values: Option<Vec<String>>,
    #[serde(rename = "default_missing_value")]
    default_missing_value: Option<String>,
    #[serde(rename = "default_missing_values", default)]
    default_missing_values: Option<Vec<String>>,
    #[serde(rename = "env")]
    env: Option<String>,
    #[serde(rename = "hide")]
    hide: Option<bool>,
    #[serde(rename = "hide_possible_values")]
    hide_possible_values: Option<bool>,
    #[serde(rename = "hide_default_value")]
    hide_default_value: Option<bool>,
    #[serde(rename = "hide_env")]
    hide_env: Option<bool>,
    #[serde(rename = "hide_env_values")]
    hide_env_values: Option<bool>,
    #[serde(rename = "hide_short_help")]
    hide_short_help: Option<bool>,
    #[serde(rename = "hide_long_help")]
    hide_long_help: Option<bool>,
}

impl ConfigArg {
    pub fn arg_type(&self) -> ConfigArgType {
        self.arg_type
    }
}

impl TryFrom<Config> for ConfigArg {
    type Error = crate::Error;

    fn try_from(value: Config) -> Result<Self, Self::Error> {
        let v = value.try_deserialize()?;
        Ok(v)
    }
}

impl TryFrom<ConfigArg> for Arg {
    type Error = crate::Error;

    fn try_from(value: ConfigArg) -> Result<Self, Self::Error> {
        let arg = Arg::new(&value.name)
            .with_some(value.help, Arg::help)
            .with_some(value.long_help, Arg::long_help)
            .with_some(value.next_line_help, Arg::next_line_help)
            .with_some(value.short, Arg::short)
            .with_some(value.long, Arg::long)
            .aliases(value.aliases)
            .short_aliases(value.short_aliases)
            .with_some(value.required, Arg::required)
            .with_some(value.trailing_var_arg, Arg::trailing_var_arg)
            .with_some(value.exclusive, Arg::exclusive)
            .with_some(value.last, Arg::last)
            .with_some(value.allow_hyphen_values, Arg::allow_hyphen_values)
            .with_some(value.allow_negative_numbers, Arg::allow_negative_numbers)
            .with_some(value.require_equals, Arg::require_equals)
            .with_some(value.require_equals, Arg::require_equals)
            .with_some(value.value_delimiter, Arg::value_delimiter)
            .with_some(value.value_terminator, Arg::value_terminator)
            .with_some(value.value_name, Arg::value_name)
            .with_some(value.default_value, Arg::default_value)
            .with_some(value.default_values, Arg::default_values)
            .with_some(value.default_missing_value, Arg::default_missing_value)
            .with_some(value.default_missing_values, Arg::default_missing_values)
            .with_some(value.env, Arg::env)
            .with_some(value.hide, Arg::hide)
            .with_some(value.hide_possible_values, Arg::hide_possible_values)
            .with_some(value.hide_default_value, Arg::hide_default_value)
            .with_some(value.hide_env, Arg::hide_env)
            .with_some(value.hide_env_values, Arg::hide_env_values)
            .with_some(value.hide_short_help, Arg::hide_short_help)
            .with_some(value.hide_long_help, Arg::hide_long_help)
            .with_some(value.raw, Arg::raw);
        // at group

        Ok(arg)
    }
}

impl_opt!(ConfigBuilder<DefaultState>);
