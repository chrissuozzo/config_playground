use anyhow::Context;
use clap::{Args, Parser};
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use serde_with::skip_serializing_none;

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
    name = "config-testing",
    version,
    about = "A utility that collects electronic part information",
    after_long_help = "Bugs can be reported at https://github.com/chrissuozzo/chipscout/issues"
)]
struct Cli {
    /// Input to process
    #[arg(long, short = 'i')]
    input: Option<String>,

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

/// Generate the app configuration
/// Up to four sources are used:
///     1. **Baseline config** : parsed at compile-time and included as a &str,
///        this config is the lowest priority but is also complete enough to
///        allow the application to function without a runtime config.
///        Lowest priority: any conflicting values from the other two sources
///        will override the baseline config.
///     2. **Runtime config** : parsed at runtime from a "./configuration"
///        directory colocated with the executable. Not guarenteed to be present,
///        but can convenient when making major deviations from baseline.
///     3. **Environment variables** : this is typically where you will find
///        API secrets, database connection params, etc.
///     4. **(Optional) CLI arguments** : only for CLI apps, these have the
///        highest priority, as they are passed in by the user upon execution.
///        These must be merged differently as the `config` crate only allows
///        file & env-var sources (no args!).
///
fn get_configuration(cli_settings: CliSettings) -> anyhow::Result<Settings> {
    static BASE_CFG: &str = include_str!("../configuration/base.yaml");
    let runtime_path = std::env::current_dir().context("Failed to determine current directory")?;
    let runtime_cfg = runtime_path.join("configuration/settings.yaml");
    // kindof hacky, but seems to be the easiest solution 🤷‍♂️
    let cli_settings_json =
        serde_json::to_string(&cli_settings).context("Couldn't parse command line args")?;

    let settings = config::Config::builder()
        .add_source(config::File::from_str(BASE_CFG, config::FileFormat::Yaml))
        .add_source(config::File::from(runtime_cfg))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
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
