use std::fs::{self, DirEntry};

use anyhow::Context;
use config::{Config, ConfigError, File, FileFormat};

use crate::Expand;

// merge_children was created to extend the Config object to allow for
// additional files to be merged into the existing config easily
pub trait MergeChildren: Sized {
    fn merge_children(self, path: &str) -> Result<Self, ConfigError>;
}

impl MergeChildren for Config {
    /// merge_children searches through the config object for a `config` attribute
    /// This attribute is expected to be an array of tables with the value `path`
    /// or `dir`. In the event both are given an error is returned. In the even neither
    /// is given an error is returned.
    ///
    /// example Toml
    ///
    /// ```toml
    /// # valid
    /// [[config]]
    ///   path = "/etc/kla/second_file.toml"
    ///
    /// # valid
    /// [[config]]
    ///   dir = "/etc/kla/config.d"
    ///
    /// # invalid
    /// [[config]]
    ///   dir = "/etc/kla/config.d"
    ///   path = "/etc/kla/second_file.toml"
    ///
    /// # invalid
    /// [[config]]
    ///   my_random_attribute = "something"
    /// ```
    fn merge_children(self, path: &str) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        let path = match self.get_array(path) {
            Ok(path) => path,
            Err(ConfigError::NotFound(_)) => return Ok(self),
            Err(err) => return Err(err),
        };

        for c in path {
            let c = c.into_table()?;
            if let (Some(_), Some(_)) = (c.get("path"), c.get("dir")) {
                return Err(ConfigError::Message(format!(
                    "config must have only `path` or `dir` property set! You set both"
                )));
            } else if let Some(path) = c.get("path") {
                // for some reason it takes ownership :(
                let path = path
                    .clone()
                    .into_string()
                    .context(format!("could not read file {}", path))
                    .map_err(|e| ConfigError::Foreign(e.into()))?
                    .shell_expansion();

                builder = builder.add_source(File::new(&path, FileFormat::Toml));
            } else if let Some(dir) = c.get("dir") {
                let dir = dir.clone().into_string()?;
                let dir = fs::read_dir(dir.as_str().shell_expansion())
                    .context(format!("could not read directory {}", dir))
                    .map_err(|e| ConfigError::Foreign(e.into()))?
                    .collect::<std::result::Result<Vec<DirEntry>, std::io::Error>>()
                    .context(format!("could not read directory {}", dir))
                    .map_err(|e| ConfigError::Foreign(e.into()))?
                    .into_iter()
                    .filter(|f| f.file_type().map(|v| v.is_file()).unwrap_or(false));

                for entry in dir {
                    let path = entry.path();
                    let path = path.as_os_str().to_string_lossy();
                    builder = builder.add_source(File::new(path.as_ref(), FileFormat::Toml));
                }
            } else {
                return Err(ConfigError::Message(format!(
                    "config must have a `path` or `dir` property set!"
                )));
            }
        }

        builder.add_source(self).build()
    }
}
