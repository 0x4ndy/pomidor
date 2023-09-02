use std::{
    error::Error,
    io,
    rc::Rc,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{prelude::*, widgets::*};
use regex::Regex;

use figlet_rs::FIGfont;

const MARGIN_LINES: usize = 2;
const INPUT_HEIGHT: usize = 3;
const SECS_IN_HOUR: u16 = 3600;
const SECS_IN_MIN: u16 = 60;

struct App {
    time_str: String,
    edit_mode: bool,
    reset: bool,
    time: Duration,
    input_str: String,
    cursor_position: usize,
}

impl App {
    fn new() -> App {
        App {
            input_str: String::from(""),
            edit_mode: false,
            reset: false,
            time: Duration::new(0, 0),
            time_str: String::from("00:00"),
            cursor_position: 0,
        }
    }

    fn on_tick(&mut self, remain: String) {
        self.time_str = remain;
    }

    fn enter_char(&mut self, new_char: char) {
        self.input_str.push(new_char);

        self.move_cursor_right();
    }

    fn submit_time(&mut self) {
        let duration = self.parse_duration(self.input_str.as_str());
        match duration {
            Some(value) => {
                self.time = value;
                self.input_str.clear();
                self.reset_cursor();
                self.reset = true;
                self.edit_mode = false;
            }
            None => {}
        }
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.cursor_position != 0;
        if is_not_cursor_leftmost {
            let current_index = self.cursor_position;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input_str.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input_str.chars().skip(current_index);
            self.input_str = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input_str.len())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.cursor_position.saturating_sub(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.cursor_position.saturating_add(1);
        self.cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn reset_cursor(&mut self) {
        self.cursor_position = 0;
    }

    fn enter_edit(&mut self) {
        self.edit_mode = true;
    }

    fn exit_edit(&mut self) {
        self.edit_mode = false;
        self.input_str.clear();
        self.reset_cursor();
    }

    fn parse_duration(&self, duration: &str) -> Option<Duration> {
        if duration.len() != 5 && duration.len() != 8 {
            return None;
        }

        let re = Regex::new(r"(:?([01][0-9]|2[0-3]):)?([0-5][0-9]):([0-5][0-9])").unwrap();
        let caps = re.captures(duration);

        match caps {
            Some(c) => {
                let h: u64 = c.get(2).map_or(0, |m| m.as_str().parse().unwrap());
                let m: u64 = c.get(3).map_or(0, |m| m.as_str().parse().unwrap());
                let s: u64 = c.get(4).map_or(0, |m| m.as_str().parse().unwrap());

                return Some(Duration::new(3600 * h + 60 * m + s, 0));
            }
            None => {
                return None;
            }
        };
    }

    fn reset(&mut self) {
        self.reset = true;
    }

    fn stop(&mut self) {
        self.time = Duration::new(0, 0);
        self.time_str = String::from("00:00");
        self.reset = true;
    }
}

fn remain_to_fmt(remain: u64) -> String {
    let (hours, minutes, seconds) = (
        remain / SECS_IN_HOUR as u64,
        (remain % SECS_IN_HOUR as u64) / SECS_IN_MIN as u64,
        remain % SECS_IN_MIN as u64,
    );

    if hours == 0 {
        format!("{:02}:{:02}", minutes, seconds)
    } else {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

fn generate_content(text: &str) -> Vec<String> {
    let mut content: Vec<String> = Vec::new();

    let standard_font = FIGfont::standard().unwrap();

    let figlet = standard_font.convert(text).unwrap();
    let letter_count = figlet.characters.len();
    let mut text_height = 0;

    if figlet.characters.len() > 0 {
        text_height = figlet.characters.get(0).unwrap().height;
    }

    for line_no in 0..text_height {
        let mut line = String::from("");
        for letter_no in 0..letter_count {
            line.push_str(
                format!(
                    "{}",
                    figlet
                        .characters
                        .get(letter_no)
                        .unwrap()
                        .characters
                        .get(line_no as usize)
                        .unwrap()
                )
                .as_str(),
            );
        }
        content.push(line);
    }
    content
}

fn create_chunks(size: Rect, top_h: u16, text_h: u16, bot_h: u16, input_h: u16) -> Rc<[Rect]> {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(top_h),
                Constraint::Length(text_h),
                Constraint::Length(bot_h),
                Constraint::Max(input_h),
            ]
            .as_ref(),
        )
        .split(size);

    chunks
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let size = f.size();
    let mut text: Vec<Line> = Vec::new();

    let content = generate_content(app.time_str.as_str());

    let text_height = content.len() + MARGIN_LINES + INPUT_HEIGHT;

    if text_height as u16 > size.height {
        return;
    }

    let blank_height: u16 = size.height - (text_height as u16);

    let top_height: u16 = blank_height / 2;
    let mut bot_height: i16 = (blank_height / 2) as i16;
    let mut input_height: u16 = 0;

    if app.edit_mode {
        bot_height = bot_height - INPUT_HEIGHT as i16;
        if bot_height < 0 {
            bot_height = 0;
        }
        input_height = INPUT_HEIGHT as u16;
    }

    for line in content {
        text.push(Line::from(line));
    }

    let chunks = create_chunks(
        size,
        top_height,
        text_height as u16,
        bot_height as u16,
        input_height,
    );

    let create_block = |title: String| {
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().fg(Color::Gray))
            .title(Span::styled(
                title,
                Style::default().add_modifier(Modifier::BOLD),
            ))
    };

    let paragraph = Paragraph::new(text.clone())
        .style(Style::default().fg(Color::Gray))
        .block(create_block(String::from("")))
        .alignment(Alignment::Center);
    f.render_widget(paragraph, chunks[1]);

    if app.edit_mode {
        let input = Paragraph::new(app.input_str.as_str())
            .style(Style::default())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Session timer (format hh:mm:ss)"),
            );
        f.render_widget(input, chunks[3]);
        f.set_cursor(
            chunks[3].x + app.cursor_position as u16 + 1,
            chunks[3].y + 1,
        );
    }
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut start = Instant::now();
    let mut deadline = Duration::new(0, 0);

    loop {
        terminal.draw(|f| ui(f, &app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if app.reset {
            app.reset = false;
            deadline = app.time;
            start = Instant::now();
        }

        if crossterm::event::poll(timeout)? {
            if app.edit_mode {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Enter => {
                                app.submit_time();
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
                            KeyCode::Esc => {
                                app.exit_edit();
                            }
                            _ => {}
                        }
                    }
                }
            } else {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('e') => {
                            app.enter_edit();
                        }
                        KeyCode::Char('r') => {
                            app.reset();
                        }
                        KeyCode::Char('s') => {
                            app.stop();
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();

            if deadline.as_secs() == 0 {
                continue;
            }

            let mut elapsed = start.elapsed();

            if deadline < elapsed {
                start = Instant::now();
                elapsed = start.elapsed();
            }
            let remain = deadline - elapsed;
            let time_str = remain_to_fmt(remain.as_secs());

            app.on_tick(time_str);
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let app = App::new();
    let res = run_app(&mut terminal, app, tick_rate);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
