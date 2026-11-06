use crate::{
    blocks::BlockConfig,
    error::{RopsError, RopsResult},
    git::{GitSettings, GithubDownloadRelease},
    settings::Settings,
    tools::strip_tag_prefix,
    utils::{StreamCommand, as_true, new_table},
};
use comfy_table::Table;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::{collections::HashMap, path::Path, process::Command};

#[derive(clap::Subcommand, Debug, Clone)]
pub enum ChartsCommand {
    /// List all available charts
    #[command(alias = "ls")]
    List {
        /// Output raw JSON instead of a table
        #[arg(long, action = clap::ArgAction::SetTrue)]
        raw: bool,
    },
    /// Update helm plugins
    Update,
    /// Deploy a chart
    Deploy {
        /// The name of the chart
        chart: String,
        /// K8s environment to deploy to
        #[arg(short, long)]
        env: Option<String>,
        /// The namespace to deploy the chart in
        #[arg(short, long)]
        namespace: Option<String>,
        /// override additional variables path
        #[arg(short, long)]
        vars: Option<String>,
        /// Additional deploy arguments
        #[arg(short, long, num_args = 1..)]
        args: Vec<String>,
        /// Additional deploy arguments
        #[arg(short, long, num_args = 1..)]
        set: Vec<String>,
        /// Deploy block only
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        block: Option<bool>,
        /// Wait for deployment to finish
        #[arg(long, action = clap::ArgAction::SetTrue)]
        wait: Option<bool>,
        /// Dry run the deployment
        #[arg(long, action = clap::ArgAction::SetTrue)]
        dry_run: Option<bool>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChartsSettings {
    /// mapping of environment to cluster names
    #[serde(default)]
    pub envs: HashMap<String, String>,
    /// location of the chart configuration yaml file
    #[serde(default = "ChartsSettings::get_default_chart_config")]
    pub config: String,
    /// optional path for additional variables and secrets
    pub vars: Option<String>,
    /// Default namespace
    #[serde(default = "ChartsSettings::get_default_namespace")]
    pub default_namespace: String,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Chart {
    pub chart: String,
    pub alias: Option<String>,
    pub namespace: Option<String>,
    pub description: Option<String>,
    #[serde(default, rename = "helm-repos")]
    pub helm_repos: HashMap<String, String>,
    #[serde(default, rename = "git-repos")]
    pub git_repos: HashMap<String, String>,
    pub block: Option<BlockConfig>,
    #[serde(default = "as_true", rename = "append-namespace")]
    pub append_namespace: bool,
}

impl Default for ChartsSettings {
    fn default() -> Self {
        Self {
            config: Self::get_default_chart_config(),
            default_namespace: Self::get_default_namespace(),
            envs: HashMap::new(),
            vars: None,
        }
    }
}

pub struct DeployChart {
    chart: String,
    config: Chart,
    cluster: String,
    namespace: String,
    wait: bool,
    dry_run: bool,
    vars: Option<String>,
    set: Vec<String>,
    args: Vec<String>,
}

impl ChartsCommand {
    /// Run the Docker command
    pub fn run(&self, settings: &Settings) -> RopsResult<()> {
        let charts = serde_yaml::from_str::<HashMap<String, Chart>>(&std::fs::read_to_string(
            &settings.charts.config,
        )?)?;
        match self {
            Self::List { raw } => {
                if *raw {
                    let json = serde_json::to_string_pretty(&charts)?;
                    println!("{}", json);
                } else {
                    println!(
                        "{}",
                        charts_table(&charts, &settings.charts.default_namespace)
                    );
                }
                Ok(())
            }
            Self::Update => ChartsSettings::install_helm_secrets(settings),
            Self::Deploy {
                chart,
                env,
                namespace,
                block,
                vars,
                set,
                args,
                wait,
                dry_run,
            } => match charts.get(chart).cloned() {
                Some(config) => {
                    if !block.unwrap_or(false) {
                        let namespace = if let Some(ns) = namespace.clone() {
                            ns
                        } else if let Some(ns) = config.namespace.clone() {
                            ns
                        } else {
                            settings.charts.default_namespace.clone()
                        };
                        let env = env.clone().unwrap_or_else(|| "prod".to_string());
                        let cluster = match settings.charts.envs.get(&env) {
                            Some(cluster) => cluster.clone(),
                            None => {
                                return Err(RopsError::Error(format!(
                                    "Environment '{env}' not found in charts settings - available are {}",
                                    settings
                                        .charts
                                        .envs
                                        .keys()
                                        .cloned()
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                )));
                            }
                        };
                        let vars = settings.charts.get_vars_path(env, vars.as_deref());
                        let deploy_chart = DeployChart {
                            chart: chart.clone(),
                            config: config.clone(),
                            namespace,
                            cluster,
                            vars,
                            wait: wait.unwrap_or_default(),
                            dry_run: dry_run.unwrap_or_default(),
                            set: set.clone(),
                            args: args.clone(),
                        };
                        deploy_chart.run()?;
                    }
                    if let Some(block_config) = config.block.as_ref() {
                        let metablock = settings.blocks.metablock()?;
                        metablock.apply(settings, block_config)?;
                    }
                    Ok(())
                }
                None => Err(RopsError::Error(format!("Chart '{}' not found", chart))),
            },
        }
    }
}

/// Render the charts as a table sorted by name
fn charts_table(charts: &HashMap<String, Chart>, default_namespace: &str) -> Table {
    let mut table = new_table([
        "Name",
        "Chart",
        "Alias",
        "Namespace",
        "Block",
        "Description",
    ]);
    let mut names: Vec<_> = charts.keys().collect();
    names.sort();
    for name in names {
        let chart = &charts[name];
        table.add_row(vec![
            name.as_str(),
            chart.chart.as_str(),
            chart.alias.as_deref().unwrap_or("-"),
            chart.namespace.as_deref().unwrap_or(default_namespace),
            if chart.block.is_some() { "yes" } else { "no" },
            chart.description.as_deref().unwrap_or(""),
        ]);
    }
    table
}

impl ChartsSettings {
    pub fn get_default_chart_config() -> String {
        std::env::var("CHARTS_CONFIG").unwrap_or_else(|_| "devops/charts/charts.yaml".to_string())
    }

    pub fn get_default_namespace() -> String {
        std::env::var("CHARTS_DEFAULT_NAMESPACE").unwrap_or_else(|_| "services".to_string())
    }

    /// Install the latest helm-secrets plugins.
    ///
    /// Helm 4 only installs signed plugins, and helm-secrets is published for
    /// it as three signed plugins on its GitHub release. They are verified
    /// with the public key of the helm-secrets author, which is built into
    /// rops rather than downloaded from the same place as the plugins.
    pub fn install_helm_secrets(settings: &Settings) -> RopsResult<()> {
        let tag = GithubDownloadRelease::new("jkroepke/helm-secrets", "secrets-{version}.tgz")
            .with_version_fn(strip_tag_prefix)
            .get_asset(settings)?
            .version;
        let version = strip_tag_prefix(&tag);
        // public key of the helm-secrets author, fingerprint
        // 1F34F95B4F30BC5B06E0D7CC3F619F17002790D8, from https://github.com/jkroepke.gpg
        let mut keyring = tempfile::NamedTempFile::new()?;
        keyring.write_all(include_bytes!("keys/helm-secrets.gpg"))?;
        for name in ["secrets", "secrets-getter", "secrets-post-renderer"] {
            // the plugin may not be installed, so a failure here is fine
            let mut command = Command::new("helm");
            command.arg("plugin").arg("uninstall").arg(name);
            StreamCommand::new(command).run()?;
            let mut command = Command::new("helm");
            command
                .arg("plugin")
                .arg("install")
                .arg("--keyring")
                .arg(keyring.path())
                .arg(format!(
                    "https://github.com/jkroepke/helm-secrets/releases/download/{tag}/{name}-{version}.tgz"
                ));
            if !StreamCommand::new(command).run()? {
                return Err(RopsError::Error(format!(
                    "Failed to install Helm plugin '{name}'"
                )));
            }
        }
        Ok(())
    }

    pub fn get_vars_path(&self, env: String, vars: Option<&str>) -> Option<String> {
        let vars = if vars.is_some() {
            vars
        } else {
            self.vars.as_deref()
        };
        vars.map(|path| {
            let path = fs::canonicalize(path)
                .unwrap_or_else(|_| path.into())
                .to_string_lossy()
                .to_string();
            format!("{path}/{env}")
        })
    }
}

impl DeployChart {
    pub fn run(&self) -> RopsResult<()> {
        // Clone git repos if they are specified
        for (repo_name, repo) in self.config.git_repos.iter() {
            GitSettings::clone_repo(repo_name, repo)?;
        }
        // Add helm repos if they are specified
        for (repo_name, repo) in self.config.helm_repos.iter() {
            self.add_helm_repo(repo_name, repo)?;
        }
        let mut command = Command::new("helm");
        //
        // if vars are given use helm secrets
        if self.vars.is_some() {
            command.env("DECRYPT_CHARTS", "true").arg("secrets");
        }
        let name_or_alias = self.config.alias.as_deref().unwrap_or(self.chart.as_str());
        let chart_name = if self.config.append_namespace {
            format!("{name_or_alias}-{}", self.namespace)
        } else {
            name_or_alias.to_string()
        };
        command
            .arg("upgrade")
            .arg(&chart_name)
            .arg(&self.config.chart)
            .arg("--install")
            .arg("--namespace")
            .arg(&self.namespace);

        if let Some(var_location) = &self.vars {
            command
                .arg("-f")
                .arg(format!("{}/values.yaml", var_location))
                .arg("-f")
                .arg(format!("{}/secrets.yaml", var_location));

            let var_repo = format!("{}/{}", var_location, self.chart);
            if Path::new(&var_repo).is_dir() {
                command
                    .arg("-f")
                    .arg(format!("{}/values.yaml", var_repo))
                    .arg("-f")
                    .arg(format!("{}/secrets.yaml", var_repo));
            }
        }
        for set in self.set.iter() {
            command.arg("--set").arg(set);
        }
        for arg in self.args.iter() {
            command.arg(arg);
        }
        if self.wait {
            command.arg("--wait");
        }
        self.fetch_cluster()?;
        if StreamCommand::new(command)
            .with_dry_run(self.dry_run)
            .run()?
        {
            Ok(())
        } else {
            Err(RopsError::Error(format!(
                "Failed to deploy Helm repo '{}'",
                chart_name
            )))
        }
    }

    pub fn fetch_cluster(&self) -> RopsResult<()> {
        let mut command = Command::new("aws");
        command
            .arg("eks")
            .arg("update-kubeconfig")
            .arg("--name")
            .arg(&self.cluster);
        if StreamCommand::new(command)
            .with_dry_run(self.dry_run)
            .run()?
        {
            Ok(())
        } else {
            Err(RopsError::Error(format!(
                "Failed to update kubeconfig for cluster '{}'",
                self.cluster
            )))
        }
    }

    pub fn add_helm_repo(&self, repo_name: &str, repo_url: &str) -> RopsResult<()> {
        let mut command = Command::new("helm");
        // --force-update re-adds the repo when it is already registered, which
        // also downloads its index. Without it, helm skips a registered repo
        // and downloads nothing, so a missing or stale index makes the deploy
        // fail with "no cached repo found".
        command
            .arg("repo")
            .arg("add")
            .arg("--force-update")
            .arg(repo_name)
            .arg(repo_url);
        if StreamCommand::new(command).run()? {
            Ok(())
        } else {
            Err(RopsError::Error(format!(
                "Failed to add Helm repo '{}'",
                repo_name
            )))
        }
    }
}
