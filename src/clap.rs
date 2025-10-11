use clap::{
    builder::{IntoResettable, OsStr},
    Arg,
};

pub trait DefaultValueIfSome {
    fn default_value_if_some(self, val: Option<impl IntoResettable<OsStr>>) -> Self;
}

impl DefaultValueIfSome for Arg {
    fn default_value_if_some(self, val: Option<impl IntoResettable<OsStr>>) -> Self {
        if let Some(default_env) = val {
            self.default_value(default_env)
        } else {
            self
        }
    }
}
