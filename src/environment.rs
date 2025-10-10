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
        if let Some(prefix) = self.prefix.as_ref() {
            let mut url = String::from(prefix.trim_end_matches("/"));
            url.push_str(uri);
            url
        } else {
            String::from(uri)
        }
    }
}
