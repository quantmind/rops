use clap::Parser;
mod blocks;
mod charts;
mod docker;
mod error;
mod extra;
mod git;
mod repo;
mod self_update;
mod settings;
mod system;
mod tools;
mod utils;
use tracing_subscriber::{EnvFilter, prelude::*};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)] // Command config
enum CliArgs {
    /// Show settings
    Settings,
    /// Run a Docker command
    #[command(subcommand)]
    Docker(docker::DockerCommand),
    /// Deploy Helm charts to Kubernetes
    #[command(subcommand)]
    Charts(charts::ChartsCommand),
    /// Manage repo and create a new git tag
    #[command(subcommand)]
    Repo(repo::RepoCommand),
    /// Self update rops to latest version from github
    SelfUpdate,
    /// Third party tools management
    #[command(subcommand)]
    Tools(tools::ToolsCommand),
    /// Other additional commands
    #[command(subcommand)]
    Extra(extra::ExtraCommand),
}

fn main() {
    dotenv::from_path(".env").ok();
    // Initialize logger with default info level if RUST_LOG is not set
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
    // run the application
    if let Err(err) = run_app() {
        log::error!("Error: {}", err);
        std::process::exit(1);
    };
}

fn run_app() -> error::RopsResult<()> {
    // Read configuration file
    let config_name = std::env::var("ROPS_CONFIG").unwrap_or_else(|_| "rops.toml".to_string());
    let settings = settings::Settings::load(&config_name);

    let app = CliArgs::parse();
    match app {
        CliArgs::Settings => match serde_json::to_string_pretty(&settings) {
            Ok(pretty_settings) => {
                println!("{}", pretty_settings);
                Ok(())
            }
            Err(err) => Err(format!("Failed to serialize settings: {}", err).into()),
        },
        CliArgs::Docker(docker) => docker.run(&settings),
        CliArgs::Charts(charts) => charts.run(&settings),
        CliArgs::Repo(repo) => repo.run(&settings),
        CliArgs::SelfUpdate => self_update::self_update(&settings),
        CliArgs::Tools(tools) => tools.run(&settings),
        CliArgs::Extra(extra) => extra.run(&settings),
    }
}
