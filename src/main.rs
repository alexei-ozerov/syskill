use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::event::{read, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::prelude::*;
use ratatui::style::palette::tailwind;
use ratatui::widgets::*;
use ratatui::Terminal;
use std::io::{self, Stdout};
use sysinfo::{Pid, ProcessExt, System, SystemExt};

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

#[derive(Clone)]
struct Data {
    name: String,
    pid: String,
    cpu_usage: String,
    memory: String,
}

impl Data {
    const fn ref_array(&self) -> [&String; 4] {
        [&self.name, &self.pid, &self.cpu_usage, &self.memory]
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn pid(&self) -> &str {
        &self.pid
    }
    fn cpu_usage(&self) -> &str {
        &self.cpu_usage
    }
    fn memory(&self) -> &str {
        &self.memory
    }
}
struct App {
    state: TableState,
    items: Vec<Data>,
    scroll_state: ScrollbarState,
    ctx: System,
    colors: TableColors,
}

const ITEM_HEIGHT: usize = 4;

impl App {
    fn new() -> Self {
        Self {
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::default(),
            items: Vec::new(),
            ctx: System::new_all(),
            colors: TableColors::new(&PALETTES[0]),
        }
    }

    pub fn clean(&mut self) {
        self.items = Vec::new();
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn set_scroll(&mut self) {
        self.scroll_state = ScrollbarState::new((self.items.len() - 1) * ITEM_HEIGHT);
    }

    pub fn get_proc(&mut self) {
        let system = &self.ctx;
        let processes = system.processes();
        let mut data_vec = Vec::new();

        for (_index, (pid, process)) in processes.iter().enumerate() {
            let name = process.name();
            let cpu_usage = process.cpu_usage().to_string();
            let memory = process.memory().to_string();
            let pid = pid.to_string();
            self.items.push(Data {
                name: name.to_string().clone(),
                pid: pid.clone(),
                cpu_usage: cpu_usage.clone(),
                memory: memory.clone(),
            });
            data_vec.push(vec![name.to_string(), pid, cpu_usage, memory]);
        }
    }

    pub fn delete_proc(&mut self) {
        let row = &self.items[self.state.selected().unwrap() as usize].pid;
        let s = System::new_all();
        if let Some(process) = s.process(Pid::from(row.parse::<i32>().unwrap())) {
            process.kill();
        }
        self.refresh();
    }

    pub fn refresh(&mut self) {
        self.ctx = System::new_all();
        self.clean();
        self.get_proc();
        self.set_scroll();
    }

    pub fn render(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) {
        let widths = [
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(10),
        ];
        let mut rows: Vec<Row> = Vec::new();
        self.items.iter().for_each(|r| {
            rows.push(Row::new(vec![
                r.name.clone(),
                r.pid.clone(),
                r.cpu_usage.clone(),
                r.memory.clone(),
            ]))
        });
        let table = Table::new(rows, widths)
            .block(Block::new().title("Processes"))
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">>")
            .block(Block::new())
            .highlight_spacing(HighlightSpacing::Always)
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            )
            .header(Row::new(vec![
                "NAME".to_string(),
                "PID".to_string(),
                "CPU USAGE".to_string(),
                "MEMORY".to_string(),
            ]));
        terminal
            .draw(|frame| {
                let area = frame.size();
                frame.render_stateful_widget(table, area, &mut self.state.clone());
                frame.set_cursor(0, 0);
            })
            .unwrap();
    }
}

fn main() {
    enable_raw_mode().unwrap();
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let options = TerminalOptions {
        viewport: Viewport::Inline(16),
    };
    let mut terminal = Terminal::with_options(backend, options).unwrap();

    let mut app = App::new();
    app.get_proc();
    app.set_scroll();

    loop {
        app.render(&mut terminal);

        if let Ok(Event::Key(key_event)) = read() {
            match key_event.code {
                KeyCode::Char('q') => break,
                KeyCode::Char('r') => {
                    app.refresh();
                }
                KeyCode::Char('k') => {
                    app.next();
                }
                KeyCode::Char('j') => {
                    app.previous();
                }
                KeyCode::Char('d') => {
                    app.delete_proc();
                }
                _ => (),
            }
        }
    }

    disable_raw_mode().unwrap();
    terminal.clear().unwrap();
    //terminal.show_cursor().unwrap();
}
