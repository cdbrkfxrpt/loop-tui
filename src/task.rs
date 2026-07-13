use std::fmt;

use enum_iterator::Sequence;
use jiff::Timestamp;
use ratatui::style::{Color, Modifier};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Sequence,
)]
pub enum Status {
    #[default]
    Open,
    Closed,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Open => "󰄰",
            Self::Closed => "󰄴",
        };
        f.write_str(label)
    }
}

impl Status {
    pub fn modifier(self) -> Modifier {
        match self {
            Self::Open => Modifier::empty(),
            Self::Closed => Modifier::CROSSED_OUT,
        }
    }
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Sequence,
)]
pub enum Priority {
    Urgent,
    High,
    #[default]
    Normal,
    Low,
    DontForget,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Urgent => "",
            Self::High => "",
            Self::Normal => "",
            Self::Low => "",
            Self::DontForget => "",
        };
        f.write_str(label)
    }
}

impl Priority {
    pub fn color(self) -> Color {
        match self {
            Self::Urgent => Color::Red,
            Self::High => Color::LightRed,
            Self::Normal => Color::White,
            Self::Low => Color::Gray,
            Self::DontForget => Color::Magenta,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub body: String,
    pub context: String,
    pub status: Status,
    pub priority: Priority,
    pub updated_at: Timestamp,
}

impl Task {
    pub fn new(body: String, context: String) -> Self {
        Self {
            id: Uuid::now_v7(),
            body,
            context,
            updated_at: Timestamp::now(),
            ..Self::default()
        }
    }

    pub fn update(&mut self, body: String, context: String) {
        self.body = body;
        self.context = context;
        self.updated_at = Timestamp::now();
    }

    pub fn rotate_priority(&mut self) {
        self.priority = self
            .priority
            .previous()
            .unwrap_or(Priority::last().expect("Priority has a lowest variant"));
        self.updated_at = Timestamp::now();
    }

    pub fn rotate_status(&mut self) {
        self.status = self
            .status
            .next()
            .unwrap_or(Status::first().expect("Status has a first variant"));
        self.updated_at = Timestamp::now();
    }
}
