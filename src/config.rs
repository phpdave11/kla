use clap::{command, Command};
use config::{
    Config, ConfigError, File, FileFormat, FileSourceFile, FileStoredFormat, Map, Source, Value,
};
use std::fmt::Debug;
use std::path::Path;

use crate::clap::{Opt, OptRes};

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

pub trait AsCommand {
    type Error;

    fn as_command(&self) -> Result<Command, Self::Error>;
}

impl AsCommand for Config {
    type Error = crate::Error;

    /// as_command turns a Config object into a Clap::Command. The functions expects the following
    /// items to be defined in the config. <sub>**Required values**</sub>
    ///
    /// - short_description: A description of what the command does
    /// - description: A longer description of what the command does
    fn as_command(&self) -> Result<Command, Self::Error> {
        let command = command!()
            .with_some(self.get_string("short_description").ok(), Command::about)
            .with_some(self.get_string("description").ok(), Command::long_about);

        Ok(command)
    }
}
