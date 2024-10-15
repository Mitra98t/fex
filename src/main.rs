#[allow(unused_imports)]
use std::io::{self, stdout, Write};

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

pub enum CopyDirType {
    Parent,
    Current,
    Child,
    Nothing,
    Path(String),
}

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c800).add_modifier(Modifier::BOLD);

#[derive(Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_highlighted: bool,
}

#[derive(Default, Clone)]
pub struct DirList {
    pub path: String,
    pub entries: Vec<DirEntry>,
    pub state: ListState,
}

pub struct File {
    pub path: String,
}

#[derive(Default)]
pub struct App {
    pub curr_dir: DirList,
    pub parent_dir: Option<DirList>,
    pub child_dir: Option<DirList>,
    pub child_file: Option<File>,

    pub exit: bool,
}

impl App {
    pub fn run(&mut self) -> io::Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        let curr_dir = std::env::current_dir().expect("Failed to get current directory");
        let parent_dir = curr_dir.parent();
        let curr_dir: String = curr_dir.to_string_lossy().to_string();
        match parent_dir {
            None => {}
            Some(parent_dir) => {
                let parent_dir: String = parent_dir.to_string_lossy().to_string();
                self.parent_dir = Some(self.read_dir(&parent_dir));
            }
        }
        self.curr_dir = self.read_dir(&curr_dir);

        self.update_child_dir();

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

        let title = Paragraph::new(self.curr_dir.path.clone()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );
        f.render_widget(title, title_area);
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
                self.select_next();
                self.update_child_dir();
            }
            KeyCode::Up => {
                self.curr_dir.state.select_previous();
                self.update_child_dir();
            }
            KeyCode::Left => {
                if let Some(parent_dir) = &self.parent_dir {
                    let parent_path = parent_dir.path.clone();
                    let parent_is_root = self.check_parent_is_root(parent_path.as_str());
                    if parent_is_root {
                        self.update_curr_dir(CopyDirType::Parent);
                        self.update_parent_dir(CopyDirType::Nothing);
                    } else {
                        self.update_curr_dir(CopyDirType::Parent);
                        self.update_parent_dir(CopyDirType::Path(
                            self.find_par_dir(&self.curr_dir.path.clone()),
                        ));
                    }
                }
            }
            KeyCode::Right => {
                let selected_entry = self.curr_dir.state.selected();
                match selected_entry {
                    None => return,
                    Some(selected_entry) => {
                        if selected_entry >= self.curr_dir.entries.len() {
                            return;
                        }
                        let selected_entry = &self.curr_dir.entries[selected_entry];
                        if selected_entry.is_dir {
                            self.update_parent_dir(CopyDirType::Current);
                            self.update_curr_dir(CopyDirType::Child);
                            self.update_child_dir();
                        } else {
                            self.update_child_dir();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn select_next(&mut self) {
        let selected_entry = self.curr_dir.state.selected();
        match selected_entry {
            None => {
                self.curr_dir.state.select(Some(0));
            }
            Some(selected_entry) => {
                if selected_entry >= self.curr_dir.entries.len() {
                    return;
                }
                self.curr_dir.state.select(Some(selected_entry + 1));
            }
        }
    }

    pub fn update_curr_dir(&mut self, copy: CopyDirType) {
        match copy {
            CopyDirType::Path(path) => {
                self.curr_dir = self.read_dir(&path);
            }
            CopyDirType::Parent => {
                if let Some(parent_dir) = &self.parent_dir {
                    self.curr_dir = parent_dir.clone();
                }
            }

            CopyDirType::Current => {
                return;
            }
            CopyDirType::Child => {
                if let Some(child_dir) = &self.child_dir {
                    self.curr_dir = child_dir.clone();
                }
            }
            _ => {}
        }
    }

    pub fn update_parent_dir(&mut self, copy: CopyDirType) {
        match copy {
            CopyDirType::Nothing => {
                self.parent_dir = None;
            }
            CopyDirType::Path(path) => {
                self.parent_dir = Some(self.read_dir(&path));
            }
            CopyDirType::Parent => {
                return;
            }
            CopyDirType::Current => {
                self.parent_dir = Some(self.curr_dir.clone());
            }
            CopyDirType::Child => {
                return;
            }
        }
    }

    pub fn update_child_dir(&mut self) {
        let selected_entry = self.curr_dir.state.selected();
        match selected_entry {
            None => return,
            Some(selected_entry) => {
                if selected_entry >= self.curr_dir.entries.len() {
                    return;
                }
                let selected_entry = &self.curr_dir.entries[selected_entry];
                if selected_entry.is_dir {
                    let dir = format!("{}/{}", self.curr_dir.path, selected_entry.name);
                    self.child_dir = Some(self.read_dir(&dir));
                    self.child_file = None;
                } else {
                    self.child_file = Some(File {
                        path: format!("{}/{}", self.curr_dir.path, selected_entry.name),
                    });
                    self.child_dir = None;
                }
            }
        }
    }

    fn find_par_dir(&self, path: &str) -> String {
        let path = std::path::Path::new(path);
        let parent = path.parent().expect("Failed to get parent directory");
        parent.to_string_lossy().to_string()
    }

    fn check_parent_is_root(&self, path: &str) -> bool {
        let path = std::path::Path::new(path);
        path.parent().is_none()
    }

    pub fn read_dir(&mut self, path: &str) -> DirList {
        let entries = std::fs::read_dir(path);
        match entries {
            Ok(entries) => {
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

                DirList {
                    path: path.to_string(),
                    entries: entries.clone(),
                    state: ListState::default(),
                }
            }
            Err(e) => DirList {
                path: "Ciao matti".to_string(),
                entries: vec![DirEntry {
                    name: e.to_string(),
                    is_dir: false,
                    is_highlighted: false,
                }],
                state: ListState::default(),
            },
        }
    }

    fn render_curr_list(&mut self, f: &mut Frame, area: Rect) {
        let entry_list = self.curr_dir.entries.iter().map(|entry| {
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

        f.render_stateful_widget(curr_dir_entry_list, area, &mut self.curr_dir.state);
    }

    fn render_parent_list(&mut self, f: &mut Frame, area: Rect) {
        if let Some(parent_dir) = &self.parent_dir {
            let entry_list = parent_dir.entries.iter().map(|entry| {
                let suffix = if entry.is_dir { "/" } else { "" };
                format!("{}{}", entry.name, suffix)
            });
            let block = Block::new()
                .title(Line::raw("parent dir List").centered())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);
            let parent_dir_entry_list = List::new(entry_list).block(block);

            f.render_stateful_widget(parent_dir_entry_list, area, &mut ListState::default());
        } else {
            let par = Paragraph::new("No parent directory").block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
            f.render_widget(par, area);
        }
    }

    fn render_child_list(&mut self, f: &mut Frame, area: Rect) {
        if let Some(child_dir) = &self.child_dir {
            let entry_list = child_dir.entries.iter().map(|entry| {
                let suffix = if entry.is_dir { "/" } else { "" };
                format!("{}{}", entry.name, suffix)
            });
            let block = Block::new()
                .title(Line::raw("child dir List").centered())
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded);
            let child_dir_entry_list = List::new(entry_list).block(block);

            f.render_stateful_widget(child_dir_entry_list, area, &mut ListState::default());
        } else if let Some(child_file) = &self.child_file {
            let par = Paragraph::new(child_file.path.clone()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            );
            f.render_widget(par, area);
        }
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
