use clap::{command, Arg, Command};
use config::{
    Config, ConfigError, File, FileFormat, FileSourceFile, FileStoredFormat, Map, Source, Value,
};
use serde::Deserialize;
use std::fmt::Debug;
use std::path::Path;

use crate::clap::{Ok, Opt};

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

#[derive(Deserialize)]
pub struct ConfigCommand {
    #[serde(skip)]
    pub name: String,

    #[serde(rename = "short_description")]
    short_description: Option<String>,

    #[serde(rename = "description")]
    description: Option<String>,

    #[serde(rename = "arg")]
    args: Vec<ConfigArg>,
}

impl ConfigCommand {
    pub fn from_config<S: Into<String>>(
        name: S,
        conf: Config,
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

#[derive(Deserialize)]
struct ConfigArg {
    #[serde(rename = "name")]
    name: String,
    #[serde(rename = "short")]
    short: Option<char>,
    #[serde(rename = "short_aliases")]
    short_aliases: Vec<char>,
    #[serde(rename = "long")]
    long: Option<String>,
    #[serde(rename = "aliases")]
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
    #[serde(rename = "default_values")]
    default_values: Option<Vec<String>>,
    #[serde(rename = "default_missing_value")]
    default_missing_value: Option<String>,
    #[serde(rename = "default_missing_values")]
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
