use std::{env, ffi::OsString, fs, sync::Arc};

use clap::{arg, command, ArgAction, ArgMatches, Command};
use config::{Config, FileFormat};
use http::Method;
use kla::{
    clap::DefaultValueIfSome,
    config::{CommandWithName, KlaTemplateConfig, OptionalFile, TemplateArgsContext},
    Endpoint, Environment, Error, FetchMany, FromEnvironment, KlaClientBuilder, KlaRequestBuilder,
    Opt, OptRender, OutputBuilder, When,
};
use log::error;
use regex::Regex;
use reqwest::{Client, ClientBuilder};
use skim::{prelude::SkimOptionsBuilder, Skim, SkimItem};
use tera::{Context, Tera};
use tokio::sync::OnceCell;

static DEFAULT_ENV: OnceCell<OsString> = OnceCell::const_new();

fn command() -> Command {
    command!()
        .arg_required_else_help(true)
        .subcommand_required(false)
        .arg(arg!(--agent <AGENT> "The header agent string").default_value("kla"))
        .arg(arg!(-e --env <ENVIRONMENT> "The environment we will run the request against").required(false).default_value_if_some(DEFAULT_ENV.get().map(|v| v.as_os_str())))
        .arg(arg!(-t --template <TEMPLATE> "The template to use when formating the output. prepending with @ will read a file."))
        .arg(arg!(--"failure-template" <TEMPLATE> "The template to use when formating the failure output. prepending with @ will read a file."))
        .arg(arg!(-o --output <FILE> "The file to write the output into"))
        .arg(arg!(--timeout <SECONDS> "The amount of time allotted for the request to finish"))
        .arg(arg!(--"basic-auth" <BASIC_AUTH> "The username and password seperated by :, a preceding @ denotes a file path."))
        .arg(arg!(--"bearer-token" <BEARER_TOKEN> "The bearer token to use in requests. A preceding @ denotes a file path."))
        .arg(arg!(-H --header <HEADER> "Specify a header The key and value should be seperated by a : (eg --header \"Content-Type: application/json\")").action(ArgAction::Append))
        .arg(arg!(-Q --query <QUERY> "Specify a query parameter The key and value should be seperated by a = (eg --query \"username=Jed\")").action(ArgAction::Append))
        .arg(arg!(-F --form <FORM> "Specify a form key=value to be passed in the form body").action(ArgAction::Append))
        .arg(arg!(-v --verbose "make it loud and proud").action(ArgAction::SetTrue))
        .arg(arg!(--dry "don't actually do anything, will automatically enable verbose").action(ArgAction::SetTrue))
        .arg(arg!(--"http-version" <HTTP_VERSION> "The version of http to send the request as").value_parser(["0.9", "1.0", "1.1", "2.0", "3.0"]))
        .arg(arg!(--"no-gzip" "Do not automatically uncompress gzip responses").action(ArgAction::SetTrue))
        .arg(arg!(--"no-brotli" "Do not automatically uncompress brotli responses").action(ArgAction::SetTrue))
        .arg(arg!(--"no-deflate" "Do not automatically uncompress deflate responses").action(ArgAction::SetTrue))
        .arg(arg!(--"max-redirects" <NUMBER> "The number of redirects allowed"))
        .arg(arg!(--"no-redirects" "Disable any redirects").action(ArgAction::SetTrue))
        .arg(arg!(--proxy <PROXY> "The proxy to use for all requests."))
        .arg(arg!(--"proxy-http" <PROXY_HTTP> "The proxy to use for http requests."))
        .arg(arg!(--"proxy-https" <PROXY_HTTPS> "The proxy to use for https requests."))
        .arg(arg!(--"proxy-auth" <PROXY_AUTH> "The username and password seperated by :."))
        .arg(arg!(--"connect-timeout" <DURATION> "The amount of time to allow for connection"))
        .arg(arg!(--certificate <CERTIFICATE_FILE> "The path to the certificate to use for requests. Accepts PEM and DER, expects files to end in .der or .pem. defaults to pem").action(ArgAction::Append))
        .arg(arg!("method-or-url": [METHOD_OR_URL] "The URL path (with an assumed GET method) OR the method if another argument is supplied"))
        .arg(arg!(url: [URL] "The URL path when a method is supplied"))
        .arg(arg!(body: [BODY] "The body of the HTTP request, if prefixed with a `@` it is treated as a file path"))
        .subcommand(
            Command::new("environments")
            .about("Show the environments that are available to you.")
            .alias("envs")
            .arg(arg!(-r --regex <STATEMENT> "A regex statement").required(false).default_value(".*"))
        )
        .subcommand(
            Command::new("switch")
            .about("Select an environment to be the current context")
            .alias("context")
            .arg(arg!(matcher: [Matcher] "A regex statement to filter down matches").required(false).default_value(".*"))
        )
}

fn client(args: &ArgMatches) -> Result<Client, Error> {
    let client = ClientBuilder::new()
        .opt_header_agent(args.get_one("agent"))?
        .gzip(
            !args
                .get_one::<bool>("no-gzip")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .brotli(
            !args
                .get_one::<bool>("no-brotli")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .deflate(
            !args
                .get_one::<bool>("no-deflate")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .connection_verbose(
            args.get_one::<bool>("verbose")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .opt_max_redirects(args.get_one("max-redirects"))
        .no_redirects(
            args.get_one::<bool>("no-redirects")
                .map(|v| *v)
                .unwrap_or_default(),
        )
        .opt_proxy(args.get_one("proxy"), args.get_one("proxy-auth"))?
        .opt_proxy_http(args.get_one("proxy-http"), args.get_one("proxy-auth"))?
        .opt_proxy_https(args.get_one("proxy-https"), args.get_one("proxy-auth"))?
        .opt_certificate(args.get_many("certificate"))?
        .build()?;
    Ok(client)
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(_) => (),
        Err(err) => error!("{}", err),
    }
}

async fn run() -> Result<(), Error> {
    colog::init();

    let conf = Config::builder()
        .add_source(OptionalFile::new("config.toml", FileFormat::Toml))
        .add_source(OptionalFile::new("/etc/kla/config.toml", FileFormat::Toml))
        .with_some(env::home_dir(), |config, dir| {
            config.add_source(OptionalFile::new(
                format!(
                    "{}/.config/kla/config.toml",
                    dir.into_os_string().to_string_lossy()
                )
                .as_str(),
                FileFormat::Toml,
            ))
        })
        .set_default("default.environment", "/etc/kla/.default-environment")?
        .build()?;

    // if the config file has a default environment we want to store it in a static
    // variable so it can be used everywhere
    if let Ok(default_environment) = fs::read_to_string(
        conf.get_string("default.environment")
            .map(String::shell_expansion)
            .expect("default value"),
    ) {
        DEFAULT_ENV
            .get_or_init(|| async { OsString::from(default_environment) })
            .await;
    }

    let m = command()
        .subcommand(
            Command::new("run")
                .about("run templates defined for the environment")
                .alias("template")
                .arg(arg!(template: [template] "The template you want to run"))
                .allow_external_subcommands(true)
                .disable_help_flag(true)
                .arg(
                    arg!([args] ... "Any arguments for the template")
                        .trailing_var_arg(true)
                        .allow_hyphen_values(true),
                )
                .arg(arg!(-h --help "Show the help command, and all templates available to you.")),
        )
        .get_matches();

    match m.subcommand() {
        Some(("environments", envs)) => run_environments(envs, &conf),
        Some(("switch", envs)) => run_switch(envs, &conf),
        Some(("run", envs)) => run_run(envs.get_one::<String>("template"), &m, &conf).await,
        _ => run_root(&m, &conf).await,
    }
}

/// run_run will exectute a template
async fn run_run<S: Into<String>>(
    template: Option<S>,
    args: &ArgMatches,
    conf: &Config,
) -> Result<(), Error> {
    let verbose = args
        .get_one::<bool>("verbose")
        .map(|v| *v)
        .unwrap_or_default();

    let template: String = match template.map(|s| s.into()) {
        None => return run_run_empty(args, conf),
        Some(template) if template == "help" => return run_run_empty(args, conf),
        Some(template) if template == "--help" => return run_run_empty(args, conf),
        Some(template) => template,
    };

    let env = Environment::new(args.get_one("env"), conf)?;
    let tmpl_config = Config::builder()
        .add_source_environment(&env, &template)?
        .build()?;

    let m = command()
        .subcommand(
            Command::new("run")
                .about("run templates defined for the environment")
                .alias("template")
                .subcommand((&tmpl_config).command_with_name(&template)?),
        )
        .get_matches();

    let tmpl_m = m
        .subcommand()
        .expect("only run in run")
        .1
        .subcommand()
        .expect("only run with template")
        .1;

    let context = Context::new().template_args(&tmpl_config, &tmpl_m)?;

    let tmpl = Tera::default().with_kla_template(&tmpl_config)?;
    let client = client(&m)?;
    let url = env.create_url(&tmpl.render("uri", &context)?);
    let method = Method::try_from(tmpl.render("method", &context)?.to_uppercase().as_str())?;

    let response = client
        .request(method, url)
        .opt_body(tmpl.render_some("body", &context)?.as_ref())?
        .opt_headers(m.get_many("header"))?
        .opt_headers(Some(tmpl.fetch_with_prefix("header", &context)))?
        .opt_bearer_auth(m.get_one("bearer-token"))
        .opt_basic_auth(m.get_one("basic-auth"))
        .opt_query(m.get_many("query"))?
        .opt_headers(Some(tmpl.fetch_with_prefix("query", &context)))?
        .opt_form(m.get_many("form"))?
        .opt_headers(Some(tmpl.fetch_with_prefix("form", &context)))?
        .opt_timeout(m.get_one("timeout"))?
        .opt_version(m.get_one("http-version"))?
        .send()
        .await?;

    let succeed = response.status().is_success();

    OutputBuilder::new(response)
        .opt_template(
            match succeed {
                true => tmpl.render_some("output", &context)?,
                false => tmpl.render_some("failure-output", &context)?,
            }
            .as_ref(),
        )?
        .opt_template(match succeed {
            true => args.get_one("template"),
            false => args.get_one("failure-template"),
        })?
        .opt_output(args.get_one("output"))
        .await?
        .when(verbose, OutputBuilder::header_prelude)
        .render()
        .await?;

    Ok(())
}

fn run_run_empty(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let env = Environment::new(args.get_one("env"), conf)?;

    let mut m = Command::new("run")
        .about("run templates defined for the environment")
        .alias("template")
        .arg_required_else_help(true);

    for template in env.templates()? {
        let tmpl_conf = Config::builder()
            .add_source_environment(&env, &template)?
            .build()?;
        m = m.subcommand(tmpl_conf.command_with_name(&template)?);
    }

    command().subcommand(m).get_matches();

    Ok(())
}

fn run_environments(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let r = Regex::new(args.get_one::<String>("regex").unwrap())?;

    let environments = conf
        .get_table("environment")?
        .into_iter()
        .filter_map(|(k, v)| if r.is_match(&k) { Some((k, v)) } else { None });

    for (k, v) in environments {
        let mut env: Endpoint = v.try_deserialize()?;
        env.name = k;
        println!("{}", env);
    }

    Ok(())
}

fn run_switch(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let (send, recv) = crossbeam_channel::unbounded();
    let r = Regex::new(args.get_one::<String>("matcher").unwrap())?;

    let environments = conf
        .get_table("environment")?
        .into_iter()
        .filter_map(|(k, v)| if r.is_match(&k) { Some((k, v)) } else { None });

    for (name, val) in environments {
        let mut endpoint: Endpoint = val.try_deserialize()?;
        endpoint.name = name;
        let endpoint: Arc<dyn SkimItem> = Arc::new(endpoint);
        send.send(endpoint).unwrap();
    }

    let options = SkimOptionsBuilder::default()
        .preview(Some(String::from("right")))
        .build()?;

    let selected = Skim::run_with(&options, Some(recv))
        .filter(|f| !f.is_abort)
        .map(|v| v.selected_items)
        .into_iter()
        .flatten()
        .next()
        .map(|v| v.text().to_string());

    if let Some(selected) = selected {
        fs::write(
            conf.get_string("default.environment")
                .map(String::shell_expansion)
                .expect("default value"),
            selected,
        )?;
    }

    Ok(())
}

// run_root will run the command with no arguments
async fn run_root(args: &ArgMatches, conf: &Config) -> Result<(), Error> {
    let env = Environment::new(args.get_one("env"), conf)?;

    let verbose = args
        .get_one::<bool>("verbose")
        .map(|v| *v)
        .unwrap_or_default();

    let (uri, method) = if let Some(uri) = args.get_one::<String>("url") {
        (
            uri,
            Method::try_from(
                args.get_one::<String>("method-or-url")
                    .expect("required")
                    .to_uppercase()
                    .as_str(),
            )?,
        )
    } else {
        (
            args.get_one("method-or-url").expect("required"),
            Method::GET,
        )
    };

    let url = env.create_url(uri);
    let client = client(args)?;

    let response = client
        .request(method, url)
        .opt_body(args.get_one("body"))?
        .opt_headers(args.get_many("header"))?
        .opt_bearer_auth(args.get_one("bearer-token"))
        .opt_basic_auth(args.get_one("basic-auth"))
        .opt_query(args.get_many("query"))?
        .opt_form(args.get_many("form"))?
        .opt_timeout(args.get_one("timeout"))?
        .opt_version(args.get_one("http-version"))?
        .send()
        .await?;

    let succeed = response.status().is_success();

    OutputBuilder::new(response)
        .opt_template(if succeed {
            args.get_one("template")
        } else {
            args.get_one("failure-template")
        })?
        .when(verbose, OutputBuilder::version_prelude)
        .when(verbose, OutputBuilder::code_prelude)
        .when(verbose, OutputBuilder::header_prelude)
        .opt_output(args.get_one("output"))
        .await?
        .render()
        .await?;

    Ok(())
}

// This trait does some string interpilation to turn paths into
// more useful paths
trait Expand {
    fn shell_expansion(self) -> Self;
}

impl Expand for String {
    // Does the following
    // replaces ~ with the home directory
    fn shell_expansion(self) -> Self {
        self.replace(
            "~",
            env::home_dir()
                .map(|b| b.to_string_lossy().to_string())
                .unwrap_or(String::from("~"))
                .as_str(),
        )
    }
}
