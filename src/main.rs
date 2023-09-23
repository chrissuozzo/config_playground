use anyhow::Context;
use clap::{Args, Parser};
use const_str::convert_ascii_case;
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use serde_with::skip_serializing_none;
use std::path::PathBuf;

#[derive(serde::Deserialize, Clone, Debug)]
struct Settings {
    somebool: bool,
    somestring: String,
    somesecret: Secret<String>,
    somestruct: SomeStructSettings,
    someoptionalstring: Option<String>,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct SomeStructSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    someint: u64,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "config_playground",
    version,
    about = "Utility to test out configuration layering",
    after_long_help = "Bugs can be reported at https://github.com/chrissuozzo/config_playground/issues"
)]
struct Cli {
    /// Optional input file to process
    #[arg(long, short = 'i')]
    input_file: Option<PathBuf>,

    #[clap(flatten)]
    cli_settings: CliSettings,
}

#[skip_serializing_none]
#[derive(serde::Serialize, Clone, Debug, Args)]
struct CliSettings {
    /// somestring setting
    #[arg(long)]
    somestring: Option<String>,
}

/// ## Generate the app configuration
///
/// Config sources listed below are eagerly loaded from lowest to highest
/// priority, with conflicts resolving to the higher priority value:
///
///     1. **Baseline config file** : Parsed at compile-time from:
///             `../configuraton/base.yaml`
///        Included as a &str, this config is complete enough to allow the
///        application to function without any runtime config file.
///     2. **Runtime config file** : Parsed at runtime from:
///             `./configuration/settings.yaml`
///        Not guarenteed to be present, but can be convenient when making
///        major deviations from baseline.
///     3. **Environment variables** : These are typically where you will find
///        API secrets, database connection params, etc. We prefix our env-vars
///        with the (shouty-snake converted) cargo-provided app name instead of
///        using the more generic "APP" to prevent collisions.
///     4. **(Optional) CLI arguments** : Only for CLI apps, these have the
///        highest priority, as they are passed in by the user upon execution.
///        The `config` crate isn't really designed to source values from
///        structs (though this would be a great `derive` macro!), so we instead
///        leverage the ability to add a 'file' from a serde-serialized JSON
///        string of our CLI settings struct. This has the added benefit of
///        stripping out any optional fields that were never set.  
///
/// See [Rain's Rust CLI recommendations][1]
/// [1]: https://rust-cli-recommendations.sunshowers.io/configuration.html
///
fn get_configuration(cli_settings: CliSettings) -> anyhow::Result<Settings> {
    static BASE_CFG: &str = include_str!("../configuration/base.yaml");
    static APP_NAME: &str = convert_ascii_case!(shouty_snake, std::env!("CARGO_PKG_NAME"));

    let runtime_path = std::env::current_dir().context("Failed to determine current directory")?;
    let runtime_cfg = runtime_path.join("configuration/settings.yaml");
    // kindof hacky, but seems to be the easiest solution 🤷‍♂️
    let cli_settings_json =
        serde_json::to_string(&cli_settings).context("Couldn't parse command line args")?;

    let settings = config::Config::builder()
        .add_source(config::File::from_str(BASE_CFG, config::FileFormat::Yaml))
        .add_source(config::File::from(runtime_cfg))
        .add_source(config::Environment::with_prefix(APP_NAME).separator("__"))
        .add_source(config::File::from_str(
            cli_settings_json.as_str(),
            config::FileFormat::Json,
        ))
        .build()
        .context("Couldn't build settings")?;

    settings
        .try_deserialize::<Settings>()
        .context("Error deserializing settings")
}

/// This program is a playground for testing configuration layering.
fn main() {
    let cli_settings = Cli::parse().cli_settings;
    let settings = get_configuration(cli_settings).expect("Failed to parse configuration");

    println!("Settings: {:?}", settings);
    println!("Secret: {}", settings.somesecret.expose_secret());
}
