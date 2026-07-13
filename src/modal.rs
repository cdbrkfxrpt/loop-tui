use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use enum_dispatch::enum_dispatch;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols::border,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget},
};
use uuid::Uuid;

use crate::store::Store;

#[enum_dispatch]
pub trait ModalControl {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn handle_keys(&mut self, key_event: KeyEvent, store: &mut Store);
    fn closed(&self) -> bool;
}

#[derive(Debug)]
#[enum_dispatch(ModalControl)]
pub enum Modal {
    EditTaskModal,
}

/// Which of the overlay's two inputs currently receives keystrokes.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
enum Field {
    #[default]
    Body,
    Context,
}

impl Field {
    fn toggle(self) -> Self {
        match self {
            Field::Body => Field::Context,
            Field::Context => Field::Body,
        }
    }
}

#[derive(Debug, Default)]
pub struct EditTaskModal {
    task_id: Uuid,
    body: String,
    context: String,
    focus: Field,
    closed: bool,
}

impl ModalControl for EditTaskModal {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let popup_area = area.centered(Constraint::Percentage(70), Constraint::Length(11));
        let title = if self.task_id == Uuid::nil() {
            " Add task "
        } else {
            " Edit task "
        };

        // Clear the whole popup once, then stack the two fields inside it. Each
        // field is 5 rows tall (border + vertical padding + one text row), so two
        // of them exactly fill the height-10 popup.
        Clear.render(popup_area, buf);
        let [body_area, context_area, ui_hint_area] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .areas(popup_area);

        self.render_field(body_area, buf, title, &self.body, Field::Body);
        self.render_field(
            context_area,
            buf,
            " Context ",
            &self.context,
            Field::Context,
        );
        Paragraph::new(
            Line::from(" [Tab] [Enter] [Esc] ")
                .style(Style::new().blue().bold())
                .centered(),
        )
        .render(ui_hint_area, buf);
    }

    fn handle_keys(&mut self, key_event: KeyEvent, store: &mut Store) {
        let KeyEvent {
            code, modifiers, ..
        } = key_event;

        if modifiers == KeyModifiers::CONTROL {
            match code {
                KeyCode::Char('w') => {
                    let s = self.focused_input_mut();
                    s.truncate(s.trim_end().len());
                    s.drain(s.rfind(' ').map_or(0, |p| p + 1)..);
                }
                KeyCode::Char('u') => self.focused_input_mut().clear(),
                _ => {}
            }
            return;
        }
        match code {
            // Move the cursor between the body and context inputs.
            KeyCode::Tab => self.focus = self.focus.toggle(),
            // Commit the task and close the overlay. We ignore an empty body so a
            // stray Enter can't create a blank task; a blank context is fine and
            // simply becomes `None`. A present `id` edits in place, `None` adds.
            KeyCode::Enter => {
                let body = self.body.trim().to_string();
                if !body.is_empty() {
                    let context = self.context.trim();
                    store.upsert(self.task_id, body, context);
                }
                self.closed = true;
            }
            // Abandon whatever was typed.
            KeyCode::Esc => self.closed = true,
            KeyCode::Backspace => {
                self.focused_input_mut().pop();
            }
            KeyCode::Char(c) => self.focused_input_mut().push(c),
            _ => {}
        }
    }

    fn closed(&self) -> bool {
        self.closed
    }
}

impl EditTaskModal {
    pub fn new_task(context: impl Into<String>) -> Modal {
        let context = context.into();
        Modal::EditTaskModal(Self {
            context,
            ..Default::default()
        })
    }

    pub fn edit_task(task_id: Uuid, store: &Store) -> Modal {
        let Some(task) = store.find_task(task_id) else {
            panic!("task with id {task_id} not found in store")
        };

        Modal::EditTaskModal(Self {
            task_id: task.id,
            body: task.body.clone(),
            context: task.context.clone(),
            ..Default::default()
        })
    }

    /// The buffer the keystrokes currently land in, picked by `focus`.
    fn focused_input_mut(&mut self) -> &mut String {
        match self.focus {
            Field::Body => &mut self.body,
            Field::Context => &mut self.context,
        }
    }

    /// Renders a single labelled input. Only the focused field shows the cursor
    /// and a highlighted border, so it is obvious where typing lands.
    fn render_field(&self, area: Rect, buf: &mut Buffer, title: &str, value: &str, field: Field) {
        let mut spans = vec![value.to_string().into()];
        let border_style = if self.focus == field {
            spans.push("█".into());
            Style::new().blue()
        } else {
            Style::new().dim()
        };

        let block = Block::bordered()
            .title(title)
            .border_set(border::THICK)
            .padding(Padding::uniform(1))
            .border_style(border_style);

        Paragraph::new(Line::from(spans).white())
            .block(block)
            .render(area, buf);
    }
}
