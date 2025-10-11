use config::Config;

pub struct Environment {
    prefix: Option<String>,
}

impl Environment {
    pub fn new(env: Option<&String>, config: &Config) -> Environment {
        let env = if let Some(env) = env {
            env
        } else {
            return Environment { prefix: None };
        };

        Environment {
            prefix: config
                .get_string(format!("environment.{}.url", env).as_ref())
                .ok(),
        }
    }

    pub fn create_url(&self, uri: &str) -> String {
        // if there is no environment we should assume the value is the url
        let prefix = if let Some(prefix) = self.prefix.as_ref() {
            prefix
        } else {
            return String::from(uri);
        };

        // if the uri starts with http or https scheme we assume the uri is
        // a url
        if uri.starts_with("http://") || uri.starts_with("https://") {
            return String::from(uri);
        }

        // we should return the prefix of the environment with the
        // uri
        let mut url = String::from(prefix.trim_end_matches("/"));
        url.push_str(uri);
        url
    }
}
