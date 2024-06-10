use crossterm::{
    event::{read, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, style::palette::tailwind, widgets::*, Terminal};
use std::io::{self, Stdout};
use sysinfo::{Pid, ProcessExt, System, SystemExt};

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::PURPLE,
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
}
struct App {
    state: TableState,
    items: Vec<Data>,
    scroll_state: ScrollbarState,
    ctx: System,
    colors: TableColors,
    color_index: usize,
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
            color_index: 0,
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

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    pub fn set_scroll(&mut self) {
        self.scroll_state = ScrollbarState::new((self.items.len() - 1) * ITEM_HEIGHT);
    }

    pub fn get_proc(&mut self) {
        self.ctx.refresh_cpu();
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
            Constraint::Length(25),
            Constraint::Length(5),
            Constraint::Length(10),
            Constraint::Length(10),
        ];
        let mut rows_pusher: Vec<Row> = Vec::new();
        self.items.iter().for_each(|r| {
            rows_pusher.push(Row::new(vec![
                r.name.clone(),
                r.pid.clone(),
                r.cpu_usage.clone(),
                r.memory.clone(),
            ]))
        });

        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(2)
        });

        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_style_fg);
        let table = Table::new(rows, widths)
            .block(Block::new().title("Processes"))
            .highlight_style(selected_style)
            .bg(self.colors.buffer_bg)
            //.highlight_symbol(">>")
            .block(Block::new())
            .highlight_spacing(HighlightSpacing::Always)
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            )
            .header(
                Row::new(vec![
                    "NAME".to_string(),
                    "PID".to_string(),
                    "CPU USAGE".to_string(),
                    "MEMORY".to_string(),
                ])
                .style(header_style),
            );
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
    app.set_colors();
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
                    app.previous();
                }
                KeyCode::Char('j') => {
                    app.next();
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
}
