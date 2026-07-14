mod app;
mod modal;
mod state;
mod store;
mod task;

use std::path::PathBuf;

use app::App;
use clap::Parser;
use directories::ProjectDirs;
use store::Format;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the data store file (overrides the default location under the system data dir).
    #[arg(long)]
    path: Option<PathBuf>,
    /// Serialize the data store as TOML instead of JSON.
    #[arg(long)]
    toml: bool,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    let format = if cli.toml { Format::Toml } else { Format::Json };

    let Some(proj_dirs) = ProjectDirs::from("org", "cdbrkfxrpt", "loop") else {
        eyre::bail!("unable to access project dirs (used for storing config and data)");
    };

    ratatui::run(|terminal| App::try_run(&proj_dirs, cli.path, format, terminal))
}
