use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::task::{Priority, Status, Task};

#[derive(Debug, Default, Clone, Copy)]
pub enum Format {
    #[default]
    Json,
    Toml,
}

#[derive(Serialize, Deserialize, Default)]
struct TomlWrapper {
    #[serde(default, rename = "task")]
    tasks: Vec<Task>,
}

#[derive(Debug, Default)]
pub struct Store {
    file: PathBuf,
    format: Format,
    store: Vec<Task>,
}

impl Store {
    pub fn try_load(file: impl Into<PathBuf>, format: Format) -> eyre::Result<Self> {
        let file = file.into();

        let store = if file.exists() {
            let contents = fs::read_to_string(&file)?;
            match format {
                Format::Json => serde_json::from_str(&contents)?,
                Format::Toml => toml::from_str::<TomlWrapper>(&contents)?.tasks,
            }
        } else {
            Vec::new()
        };

        Ok(Self {
            file,
            format,
            store,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn find_task(&self, task_id: Uuid) -> Option<&Task> {
        self.store.iter().find(|t| t.id == task_id)
    }

    pub fn find_task_mut(&mut self, task_id: Uuid) -> Option<&mut Task> {
        self.store.iter_mut().find(|t| t.id == task_id)
    }

    pub fn tasks_by_most_recently_updated_at(&self) -> Vec<Task> {
        let mut store = self.store.clone();
        store.sort_by_key(|t| t.id);
        store
    }

    pub fn tasks_by_context(&self) -> Vec<(String, Vec<Task>)> {
        let mut map: BTreeMap<String, Vec<Task>> = BTreeMap::new();
        for t in &self.store {
            if let Some(vec) = map.get_mut(&t.context) {
                vec.push(t.clone());
            } else {
                map.insert(t.context.clone(), vec![t.clone()]);
            }
        }
        for ts in map.values_mut() {
            ts.sort_by_key(|t| t.priority);
        }
        map.into_iter().collect()
    }

    pub fn tasks_by_priority(&self) -> Vec<(String, Vec<Task>)> {
        let mut map: BTreeMap<Priority, Vec<Task>> = BTreeMap::new();
        for t in &self.store {
            let priority = t.priority;
            if let Some(vec) = map.get_mut(&priority) {
                vec.push(t.clone());
            } else {
                map.insert(priority, vec![t.clone()]);
            }
        }
        map.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    pub fn tasks_by_status(&self) -> Vec<(String, Vec<Task>)> {
        let mut map: BTreeMap<Status, Vec<Task>> = BTreeMap::new();
        for t in &self.store {
            let status = t.status;
            if let Some(vec) = map.get_mut(&status) {
                vec.push(t.clone());
            } else {
                map.insert(status, vec![t.clone()]);
            }
        }
        for ts in map.values_mut() {
            ts.sort_by_key(|t| t.priority);
        }
        map.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    pub fn upsert(&mut self, task_id: Uuid, body: impl Into<String>, context: impl Into<String>) {
        let (body, context) = (body.into(), context.into());
        if let Some(task) = self.find_task_mut(task_id) {
            task.update(body, context);
        } else {
            self.store.push(Task::new(body, context));
        }
        self.commit_to_disk();
    }

    pub fn rotate_priority(&mut self, task_id: Uuid) {
        if let Some(task) = self.find_task_mut(task_id) {
            task.rotate_priority();
            self.commit_to_disk();
        }
    }

    pub fn rotate_status(&mut self, task_id: Uuid) {
        if let Some(task) = self.find_task_mut(task_id) {
            task.rotate_status();
            self.commit_to_disk();
        }
    }

    pub fn delete(&mut self, task_id: Uuid) {
        if let Some(pos) = self.store.iter().position(|t| t.id == task_id) {
            self.store.remove(pos);
            self.commit_to_disk();
        }
    }

    fn commit_to_disk(&self) {
        let contents = match self.format {
            Format::Json => {
                serde_json::to_string_pretty(&self.store).expect("can serialize points to JSON")
            }
            Format::Toml => {
                let wrapper = TomlWrapper {
                    tasks: self.store.clone(),
                };
                toml::to_string_pretty(&wrapper).expect("can serialize points to TOML")
            }
        };
        fs::write(&self.file, contents).expect("can write serialized points to file");
    }
}
