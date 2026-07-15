mod app;
mod modal;
mod state;
mod store;
mod task;

use std::path::PathBuf;

use app::App;
use clap::Parser;
use directories::ProjectDirs;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    /// Path to the data store file (overrides the default location under the system data dir).
    #[arg(long)]
    path: Option<PathBuf>,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    let Some(proj_dirs) = ProjectDirs::from("org", "cdbrkfxrpt", "loop") else {
        eyre::bail!("unable to access project dirs (used for storing config and data)");
    };

    ratatui::run(|terminal| App::try_run(&proj_dirs, cli.path, terminal))
}
