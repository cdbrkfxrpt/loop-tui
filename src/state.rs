use std::{fs, path::PathBuf};

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Default, Clone, Serialize, Deserialize, Sequence)]
pub enum SortBy {
    #[default]
    Context,
    Priority,
    Status,
    UpdatedAt,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(skip)]
    view_state_file: PathBuf,
    pub selected: Uuid,
    pub sort_by: SortBy,
    pub scroll: u16,
}

impl State {
    pub fn try_load(view_state_file: impl Into<PathBuf>) -> eyre::Result<Self> {
        let view_state_file = view_state_file.into();

        let mut view_state = if view_state_file.exists() {
            serde_json::from_reader(fs::File::open(&view_state_file)?)?
        } else {
            Self::default()
        };
        view_state.view_state_file = view_state_file;

        Ok(view_state)
    }

    pub fn rotate_sort_by(&mut self) {
        self.sort_by = self
            .sort_by
            .next()
            .unwrap_or(SortBy::first().expect("SortBy enum has a first variant"));
    }

    pub fn commit_to_disk(&self) {
        fs::write(
            &self.view_state_file,
            serde_json::to_string_pretty(&self).expect("can serialize state to JSON"),
        )
        .unwrap_or_else(|_| {
            panic!(
                "can write JSON to state file '{}'",
                self.view_state_file.display()
            )
        });
    }
}
