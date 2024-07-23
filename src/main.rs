use crossterm::{
    event::{read, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, style::palette::tailwind, widgets::*, Terminal};
use std::io::{self, Stdout};
use sysinfo::{Pid, System};

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

#[derive(Debug)]
enum AppState {
    ProcessMode,
    SearchMode,
}

struct App {
    state: TableState,
    items: Vec<Data>,
    scroll_state: ScrollbarState,
    ctx: System,
    colors: TableColors,
    color_index: usize,
    show_popup: bool,
    mode: AppState,
    input: String,
    messages: Vec<String>,
    character_index: usize,
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
            show_popup: false,
            mode: AppState::ProcessMode,
            input: String::new(),
            messages: Vec::new(),
            character_index: 0,
        }
    }

    pub fn clean(&mut self) {
        self.items = Vec::new();
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    pub fn byte_index(&mut self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    pub fn submit_message(&mut self) {
        self.messages.push(self.input.clone());

        // TODO (ozerova): Add search function
        self.search();

        self.input.clear();
        self.reset_cursor();
    }

    pub fn search(&mut self) {
        let msg = self.input.clone();
        let procn = self.items.clone();

        let mut parsed_processes = Vec::new();
        procn.iter().for_each(|proc| {
            if proc.name.contains(&msg) {
                parsed_processes.push(proc.clone());
            }
        });

        self.items = parsed_processes.clone();
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

        self.items
            .sort_by_key(|obj| obj.pid.parse::<i32>().unwrap());
    }

    pub fn delete_proc(&mut self) {
        let row = &self.items[self.state.selected().unwrap() as usize].pid;
        let s = System::new_all();
        if let Some(process) = s.process(Pid::from(row.parse::<usize>().unwrap())) {
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

                let vertical = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Min(3)
                ]);
                let [help_area, table_area] = vertical.areas(area);

                frame.render_stateful_widget(table, table_area, &mut self.state.clone());
                frame.set_cursor(0, 0);

                let msg = vec![
                    "\n".into(),
                    "Press ".into(),
                    "/".bold(),
                    " to toggle search, press ".into(),
                    "enter".bold(),
                    " to confirm search, press ".into(),
                    "r".bold(),
                    " to refresh process list, press ".into(),
                    "d".bold(),
                    " to delete selected process, press ".into(),
                    "q".bold(),
                    " to exit.".into(),
                ];

                let text = Text::from(Line::from(msg));
                frame.render_widget(Paragraph::new(text).style(Style::default()), help_area);

                // Popup logic
                if self.show_popup {
                    let block = Block::bordered().title("Search");
                    let area = centered_rect(60, 20, area);

                    let input = Paragraph::new(self.input.as_str()).style(match self.mode {
                        AppState::ProcessMode => Style::default(),
                        AppState::SearchMode => Style::default().fg(Color::Yellow),
                    });

                    let inner_area = block.inner(area);

                    frame.render_widget(Clear, area); //this clears out the background
                    frame.render_widget(block, area);
                    frame.render_widget(input, inner_area);
                }
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
            match app.mode {
                AppState::ProcessMode => match key_event.code {
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
                    KeyCode::Char('/') => {
                        app.mode = AppState::SearchMode;
                        app.show_popup = !app.show_popup
                    }
                    _ => (),
                },
                AppState::SearchMode if key_event.kind == KeyEventKind::Press => {
                    match key_event.code {
                        KeyCode::Char('/') => {
                            app.mode = AppState::ProcessMode;
                            app.show_popup = !app.show_popup
                        }
                        KeyCode::Enter => {
                            app.submit_message();
                            app.mode = AppState::ProcessMode;
                            app.show_popup = !app.show_popup
                        }
                        KeyCode::Char(to_insert) => {
                            app.enter_char(to_insert);
                        }
                        KeyCode::Backspace => {
                            app.delete_char();
                        }
                        KeyCode::Left => {
                            app.move_cursor_left();
                        }
                        KeyCode::Right => {
                            app.move_cursor_right();
                        }
                        _ => (),
                    }
                }
                AppState::SearchMode => {}
            }
        }
    }

    disable_raw_mode().unwrap();
    terminal.clear().unwrap();
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
