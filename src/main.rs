mod app;
mod modal;
mod state;
mod store;
mod task;

use app::App;
use directories::ProjectDirs;

fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let Some(proj_dirs) = ProjectDirs::from("org", "cdbrkfxrpt", "loop") else {
        eyre::bail!("unable to access project dirs (used for storing config and data)");
    };

    ratatui::run(|terminal| App::try_run(&proj_dirs, terminal))
}
