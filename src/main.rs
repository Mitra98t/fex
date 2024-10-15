#[allow(unused_imports)]
use std::io::{self, stdout};

#[allow(unused_imports)]
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    crossterm::{
        event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
        ExecutableCommand,
    },
    layout::{Constraint, Layout, Rect},
    style::{
        palette::tailwind::{
            BLUE, CYAN, GRAY, GREEN, INDIGO, ORANGE, PINK, PURPLE, RED, SLATE, TEAL,
        },
        Color, Modifier, Style, Stylize,
    },
    text::Line,
    widgets::{Block, BorderType, Borders, List, ListState, Paragraph, StatefulWidget},
    Frame, Terminal,
};

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_highlighted: bool,
}

#[derive(Default, Clone)]
pub struct DirList {
    pub entries: Vec<DirEntry>,
    pub state: ListState,
}

#[derive(Default)]
pub struct App {
    pub current_dir: String,
    pub curr_dir_list: DirList,
    pub parent_dir: String,
    pub parent_dir_list: DirList,
    pub child_dir: String,
    pub child_dir_list: DirList,

    pub exit: bool,
}

impl App {
    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        self.current_dir = std::env::current_dir()
            .expect("Failed to get current directory")
            .to_string_lossy()
            .to_string();
        self.parent_dir = std::env::current_dir()
            .expect("Failed to get current directory")
            .parent()
            .expect("Failed to get parent directory")
            .to_string_lossy()
            .to_string();
        self.curr_dir_list = self.read_dir(".");
        self.parent_dir_list = self.read_dir(&self.parent_dir.to_string());

        while !self.exit {
            terminal.draw(|f| self.draw_ui(f))?;
            if let Event::Key(key) = event::read()? {
                self.handle_events(key);
            }
        }
        Ok(())
    }

    pub fn draw_ui(&mut self, f: &mut Frame) {
        let [title_area, main_area] =
            Layout::vertical([Constraint::Length(5), Constraint::Min(0)]).areas(f.area());
        let [left_area, center_area, right_area] = Layout::horizontal([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .areas(main_area);

        f.render_widget(Block::bordered().title("Title Bar"), title_area);
        // f.render_stateful_widget(dir_list, center_area, &mut self.curr_dir_list.state);
        self.render_parent_list(f, left_area);
        self.render_curr_list(f, center_area);
        self.render_child_list(f, right_area);
    }

    pub fn handle_events(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Down => {
                self.curr_dir_list.state.select_next();
                self.update_child_dir();
            }
            KeyCode::Up => {
                self.curr_dir_list.state.select_previous();
                self.update_child_dir();
            }
            KeyCode::Left => {
                self.current_dir = self.parent_dir.clone();
                self.parent_dir = std::path::Path::new(&self.current_dir)
                    .parent()
                    .expect("Failed to get parent directory")
                    .to_string_lossy()
                    .to_string();
                self.curr_dir_list = self.parent_dir_list.clone();
                self.parent_dir_list = self.read_dir(&self.parent_dir.clone());
                self.update_child_dir();
            }
            KeyCode::Right => {
                let selected_entry = self.curr_dir_list.state.selected().unwrap();
                let selected_entry = &self.curr_dir_list.entries[selected_entry];
                if selected_entry.is_dir {
                    self.parent_dir = self.current_dir.clone();
                    self.current_dir = format!("{}/{}", self.current_dir, selected_entry.name);
                    self.parent_dir_list = self.curr_dir_list.clone();
                    self.curr_dir_list = self.read_dir(&self.current_dir.clone());
                }
                self.update_child_dir();
            }
            _ => {}
        }
    }

    pub fn update_child_dir(&mut self) {
        let selected_entry = self.curr_dir_list.state.selected();
        match selected_entry {
            None => return,
            Some(selected_entry) => {
                if selected_entry >= self.curr_dir_list.entries.len() {
                    return;
                }
                let selected_entry = &self.curr_dir_list.entries[selected_entry];
                if selected_entry.is_dir {
                    self.child_dir = format!("{}/{}", self.current_dir, selected_entry.name);
                    self.child_dir_list = self.read_dir(&self.child_dir.clone());
                }
            }
        }
    }

    pub fn read_dir(&mut self, path: &str) -> DirList {
        let entries = std::fs::read_dir(path).expect("Failed to read directory");

        let mut entries = entries
            .map(|entry| {
                let entry = entry.expect("Failed to read entry");
                let is_dir = entry.file_type().expect("Failed to get file type").is_dir();
                DirEntry {
                    name: entry
                        .file_name()
                        .into_string()
                        .expect("Failed to get file name"),
                    is_dir,
                    is_highlighted: false,
                }
            })
            .collect::<Vec<DirEntry>>();

        entries.sort_unstable_by_key(|entry| (!entry.is_dir, entry.name.clone()));
        entries[0].is_highlighted = true;

        DirList {
            entries: entries.clone(),
            state: ListState::default(),
        }
    }

    fn render_curr_list(&mut self, f: &mut Frame, area: Rect) {
        let entry_list = self.curr_dir_list.entries.iter().map(|entry| {
            let suffix = if entry.is_dir { "/" } else { "" };
            format!("{}{}", entry.name, suffix)
        });
        let block = Block::new()
            .title(Line::raw("cur dir List").centered())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let curr_dir_entry_list = List::new(entry_list)
            .block(block)
            .highlight_style(SELECTED_STYLE);

        f.render_stateful_widget(curr_dir_entry_list, area, &mut self.curr_dir_list.state);
    }

    fn render_parent_list(&mut self, f: &mut Frame, area: Rect) {
        let entry_list = self.parent_dir_list.entries.iter().map(|entry| {
            let suffix = if entry.is_dir { "/" } else { "" };
            format!("{}{}", entry.name, suffix)
        });
        let block = Block::new()
            .title(Line::raw("parent dir List").centered())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let parent_dir_entry_list = List::new(entry_list).block(block);

        f.render_stateful_widget(parent_dir_entry_list, area, &mut self.parent_dir_list.state);
    }
    fn render_child_list(&mut self, f: &mut Frame, area: Rect) {
        let entry_list = self.child_dir_list.entries.iter().map(|entry| {
            let suffix = if entry.is_dir { "/" } else { "" };
            format!("{}{}", entry.name, suffix)
        });
        let block = Block::new()
            .title(Line::raw("parent dir List").centered())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded);
        let parent_dir_entry_list = List::new(entry_list).block(block);

        f.render_stateful_widget(parent_dir_entry_list, area, &mut self.child_dir_list.state);
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;

    stdout().execute(EnterAlternateScreen)?;

    let app_result = App::default().run();

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    app_result
}
