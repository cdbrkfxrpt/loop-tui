use std::{
    fs, io,
    path::{Path, PathBuf},
};

use ansi_to_tui::IntoText;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use directories::ProjectDirs;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Position, Rect},
    style::{Modifier, Style, Stylize},
    symbols::border,
    text::{Line, Span},
    widgets::{
        Block, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Table, Widget,
    },
};
use uuid::Uuid;

use crate::{
    modal::{EditTaskModal, Modal, ModalControl},
    state::{SortBy, State},
    store::Store,
    task::{Status, Task},
};

pub const LOOP: &str = include_str!("loop.txt");

#[derive(Debug, Default)]
pub struct App {
    state: State,
    store: Store,
    modal: Option<Modal>,
    exited: bool,
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [banner_area, content_area] =
            Layout::vertical([Constraint::Length(9), Constraint::Min(0)]).areas(area);

        // TOP BANNER --------------------------------------------------------------------------- //
        // The art carries raw ANSI colour codes, so we parse them into styled ratatui `Text` rather
        // than letting the escape bytes render literally.
        let banner_art = LOOP
            .into_text()
            .expect("LOOP is a compile-time constant containing valid ANSI");

        Paragraph::new(banner_art)
            .centered()
            .block(Block::default())
            .render(banner_area, buf);

        // MAIN CONTENT ------------------------------------------------------------------------- //
        // If no task is selected, or the selected one no longer exists, we auto-select the one that
        // was updated last.
        if !self.store.is_empty()
            && (self.store.find_task(self.state.selected).is_none()
                || self.modal.as_ref().is_some_and(ModalControl::closed))
        {
            self.modal = None;
            self.state.selected = self
                .store
                .tasks_by_most_recently_updated_at()
                .first()
                .expect("non-empty store has most recently updated element")
                .id;
        }
        let block = Block::bordered()
            .title_bottom(self.build_menu())
            .border_set(border::THICK);

        // Draw the frame first, then fill its interior. `inner` is everything inside the borders.
        let inner = block.inner(content_area);
        block.render(content_area, buf);
        self.render_tasks(inner, buf);

        // MODALS ------------------------------------------------------------------------------- //
        // The modals are drawn last so they float on top of the UI
        if let Some(modal) = &mut self.modal {
            modal.render(content_area, buf);
        }
    }
}

impl App {
    pub fn try_run(
        proj_dirs: &ProjectDirs,
        store_path: Option<PathBuf>,
        terminal: &mut DefaultTerminal,
    ) -> eyre::Result<()> {
        // Helper function to prep the directories and files
        fn set_up_file(path: &Path) -> eyre::Result<PathBuf> {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
                && !parent.exists()
            {
                fs::create_dir_all(parent)?;
            }
            Ok(path.to_path_buf())
        }

        let state_file = set_up_file(&proj_dirs.cache_dir().join("state.json"))?;
        let state = State::try_load(state_file)?;

        let store_file =
            set_up_file(&store_path.unwrap_or_else(|| proj_dirs.data_dir().join("store.json")))?;
        let store = Store::try_load(&store_file)?;

        let mut app = Self {
            state,
            store,
            ..Default::default()
        };

        while !app.exited {
            terminal.draw(|frame| frame.render_widget(&mut app, frame.area()))?;
            app.handle_key_events()?;
        }
        Ok(())
    }

    fn build_menu(&self) -> Line<'_> {
        let mut menu = vec![" [A]dd ".blue()];
        if !self.store.is_empty() {
            let selected_task = self
                .store
                .find_task(self.state.selected)
                .expect("a task is always selected");

            menu.extend_from_slice(&[
                "|".white(),
                " [E]dit ".green(),
                "|".white(),
                " [P]riority ".green(),
            ]);

            if selected_task.status == Status::Open {
                menu.extend_from_slice(&["|".white(), " [space] Close ".green()]);
            } else if selected_task.status == Status::Closed {
                menu.extend_from_slice(&[
                    "|".white(),
                    " [space] Reopen ".green(),
                    "|".white(),
                    " [D]elete ".green(),
                ]);
            }
            menu.extend_from_slice(&["|".white(), " [S]ort By ".yellow()]);
        }
        menu.extend_from_slice(&["|".white(), " [esc] quit ".red()]);

        Line::from(menu).centered()
    }

    fn render_tasks(&mut self, area: Rect, buf: &mut Buffer) {
        if self.store.is_empty() {
            Paragraph::new("no points yet".dim())
                .centered()
                .render(area.inner(Margin::new(0, 1)), buf);
            return;
        }

        let content_area = area.centered_horizontally(Constraint::Percentage(80));

        // You produce the grouping; each box is 2 border rows + one row per task.
        let sections = self.sections();
        let total: u16 = sections.iter().map(|(_, t)| t.len() as u16 + 2).sum();
        let viewport = content_area.height;

        // Paint every box into a buffer tall enough for all of them, then copy the
        // visible slice onto the screen.
        let mut scratch = Buffer::empty(Rect::new(0, 0, content_area.width, total.max(viewport)));
        let mut y = 0;
        for (title, tasks) in sections {
            let height = tasks.len() as u16 + 2;
            self.render_task_box(
                title,
                &tasks,
                Rect::new(0, y, content_area.width, height),
                &mut scratch,
            );
            y += height;
        }

        self.state.scroll = self.state.scroll.min(total.saturating_sub(viewport));
        for row in 0..viewport {
            for col in 0..content_area.width {
                if let (Some(src), Some(dst)) = (
                    scratch.cell(Position::new(col, row + self.state.scroll)),
                    buf.cell_mut(Position::new(content_area.x + col, content_area.y + row)),
                ) {
                    *dst = src.clone();
                }
            }
        }

        if total > viewport {
            let scrollbar_area = Rect::new(area.right() - 1, area.y, 1, area.height);
            let mut state = ScrollbarState::new(total as usize)
                .viewport_content_length(viewport as usize)
                .position(self.state.scroll as usize);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .render(scrollbar_area, buf, &mut state);
        }
    }

    fn render_task_box(&self, title: Line<'_>, tasks: &[Task], area: Rect, buf: &mut Buffer) {
        let rows = tasks.iter().map(|task| {
            let marker = if task.id == self.state.selected {
                "󰧂"
            } else {
                ""
            };
            Row::new([
                Cell::from(marker),
                Cell::from(task.status.to_string()),
                Cell::from(task.priority.to_string()).fg(task.priority.color()),
                Cell::from(Span::styled(
                    task.body.as_str(),
                    Style::default().add_modifier(task.status.modifier()),
                )),
            ])
        });

        let widths = [
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ];

        let table = Table::new(rows, widths)
            .column_spacing(2)
            .block(Block::new().title(title));
        Widget::render(table, area, buf);
    }

    fn sections(&self) -> Vec<(Line<'_>, Vec<Task>)> {
        match self.state.sort_by {
            SortBy::Context => self
                .store
                .tasks_by_context()
                .into_iter()
                .map(|(k, v)| {
                    (
                        Line::styled(k, Style::default().add_modifier(Modifier::BOLD)),
                        v,
                    )
                })
                .collect(),
            SortBy::Priority => self
                .store
                .tasks_by_priority()
                .into_iter()
                .map(|(k, v)| {
                    (
                        Line::styled(
                            k.to_string(),
                            Style::default().add_modifier(Modifier::BOLD).fg(k.color()),
                        ),
                        v,
                    )
                })
                .collect(),
            SortBy::Status => self
                .store
                .tasks_by_status()
                .into_iter()
                .map(|(k, v)| (Line::from(k.to_string().bold()), v))
                .collect(),
            SortBy::UpdatedAt => {
                vec![(
                    Line::styled(
                        "in order of most recent update",
                        Style::default().fg(ratatui::style::Color::DarkGray),
                    ),
                    self.store.tasks_by_most_recently_updated_at(),
                )]
            }
        }
    }

    fn ids_in_section_order(&self) -> Vec<Uuid> {
        self.sections()
            .into_iter()
            .flat_map(|(_, v)| v)
            .map(|t| t.id)
            .collect()
    }

    fn select_next(&mut self) {
        let ids = self.ids_in_section_order();
        self.state.selected = *ids
            .iter()
            .skip_while(|&id| *id != self.state.selected)
            .nth(1)
            .unwrap_or(ids.first().expect("ids list has a first element"));
    }

    fn select_previous(&mut self) {
        let ids = self.ids_in_section_order();
        self.state.selected = *ids
            .iter()
            .rev()
            .skip_while(|&id| *id != self.state.selected)
            .nth(1)
            .unwrap_or(ids.last().expect("ids list has a last element"));
    }

    fn handle_key_events(&mut self) -> io::Result<()> {
        // it's important to check that the event is a key press event as
        // crossterm also emits key release and repeat events on Windows.
        if let Event::Key(key_event) = event::read()?
            && key_event.kind == KeyEventKind::Press
        {
            // While the overlay is open it captures every keystroke — otherwise
            // typing a point that contains 'q' would quit the app mid-entry.
            if let Some(modal) = self.modal.as_mut() {
                modal.handle_keys(key_event, &mut self.store);
            } else {
                self.handle_keys(key_event);
            }
        }
        Ok(())
    }

    /// Handles keys for the main list view.
    fn handle_keys(&mut self, key_event: KeyEvent) {
        let KeyEvent {
            code, modifiers, ..
        } = key_event;

        if modifiers == KeyModifiers::CONTROL {
            match code {
                KeyCode::Char('e') => self.state.scroll = self.state.scroll.saturating_add(1),
                KeyCode::Char('y') => self.state.scroll = self.state.scroll.saturating_sub(1),
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Char('a') => {
                let context = self
                    .store
                    .find_task(self.state.selected)
                    .map(|t| t.context.clone())
                    .unwrap_or_default();
                self.modal = Some(EditTaskModal::new_task(context));
            }
            KeyCode::Esc => {
                self.state.commit_to_disk();
                self.exited = true;
            }
            _ => {}
        }

        if !self.store.is_empty() {
            match code {
                KeyCode::Char('e') => {
                    self.modal = Some(EditTaskModal::edit_task(self.state.selected, &self.store));
                }
                KeyCode::Char('p') => self.store.rotate_priority(self.state.selected),
                KeyCode::Char('s') => self.state.rotate_sort_by(),
                KeyCode::Down | KeyCode::Char('j') => self.select_next(),
                KeyCode::Up | KeyCode::Char('k') => self.select_previous(),
                _ => {}
            }
        }

        if let Some(task) = self.store.find_task(self.state.selected).cloned() {
            match code {
                KeyCode::Char(' ') => {
                    self.store.rotate_status(task.id);
                }
                KeyCode::Char('d') if task.status == Status::Closed => {
                    let ids = self.ids_in_section_order();
                    if &task.id == ids.last().expect("one ID must exist here") {
                        self.select_previous();
                    } else {
                        self.select_next();
                    }
                    self.store.delete(task.id);
                }
                _ => {}
            }
        }
    }
}
