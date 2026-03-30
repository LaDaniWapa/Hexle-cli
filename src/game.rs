use crossterm::event;
use crossterm::event::{Event, KeyCode};
use rand::RngExt;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Direction, Line};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{Frame, Terminal};
use std::io::{Result, Stdout};
use std::time::Duration;

pub struct Game {
    pub color: String,
    pub current_attempt: String,
    pub history: [Option<Guess>; 5],
    pub history_count: usize,
    pub state: GameState,
}

pub struct Guess {
    pub input: String,
    pub feedback: [DigitFeedback; 6],
}

#[derive(Clone, Copy)]
pub enum DigitFeedback {
    Correct,
    Close,
    Wrong,
}

#[derive(PartialEq)]
pub enum GameState {
    Playing,
    Won,
    Lost,
    Exit,
}

impl Game {
    pub fn new() -> Game {
        let mut rng = rand::rng();
        let r: u8 = rng.random();
        let g: u8 = rng.random();
        let b: u8 = rng.random();

        Game {
            color: format!("{:02X}{:02X}{:02X}", r, g, b),
            current_attempt: "".to_string(),
            history: [const { None }; 5],
            history_count: 0,
            state: GameState::Playing,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        while self.state != GameState::Exit {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key.code)
                }
            }
        }

        Ok(())
    }

    pub fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if self.state == GameState::Lost || self.state == GameState::Won {
                    *self = Game::new()
                }
            }
            KeyCode::Char(c) => {
                if self.state != GameState::Playing {
                    return;
                }

                if c.to_ascii_uppercase().is_ascii_hexdigit() && self.current_attempt.len() < 6 {
                    self.current_attempt.push(c.to_ascii_uppercase());
                }
            }
            KeyCode::Backspace => {
                self.current_attempt.pop();
            }
            KeyCode::Enter => {
                if self.state != GameState::Playing {
                    return;
                }

                if self.current_attempt.len() == 6 {
                    self.history[self.history_count] = Some(Guess {
                        input: self.current_attempt.clone(),
                        feedback: self.evaluate_guess(),
                    });

                    if self.current_attempt == self.color {
                        self.state = GameState::Won;
                    } else if self.history_count >= 4 {
                        self.state = GameState::Lost;
                    }

                    self.current_attempt.clear();
                    self.history_count += 1;
                }
            }
            KeyCode::Esc => {
                self.state = GameState::Exit;
            }
            _ => {}
        }
    }

    fn evaluate_guess(&self) -> [DigitFeedback; 6] {
        let mut feedback = [DigitFeedback::Wrong; 6];

        for (i, c) in self.current_attempt.chars().enumerate() {
            let target = self.color.chars().nth(i).unwrap();

            let guess_val = c.to_digit(16).unwrap();
            let target_val = target.to_digit(16).unwrap();

            feedback[i] = if guess_val == target_val {
                DigitFeedback::Correct
            } else if guess_val.abs_diff(target_val) <= 2 {
                DigitFeedback::Close
            } else {
                DigitFeedback::Wrong
            };
        }

        feedback
    }

    pub fn draw(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5), // Color block
                Constraint::Min(0),    // History
                Constraint::Length(3), // Input
            ])
            .split(frame.area());

        self.draw_color(frame, chunks[0]);
        self.draw_history(frame, chunks[1]);
        self.draw_input(frame, chunks[2]);

        if self.state != GameState::Playing {
            let popup_area = Self::centered_rect(60, 50, frame.area());
            self.draw_game_over(frame, popup_area);
        }
    }

    fn draw_color(&self, frame: &mut Frame, area: Rect) {
        let color = u32::from_str_radix(&self.color, 16).unwrap();
        let block = Block::default().style(Style::default().bg(Color::from_u32(color)));
        frame.render_widget(block, area);
    }

    fn draw_history(&self, frame: &mut Frame, area: Rect) {
        // Block with borders
        let block = Block::default().borders(Borders::ALL).title(" History ");
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Five guesses => 5 Rows
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1); 5])
            .split(inner);

        // For each Guess/Row => 1 Square + Text
        for (i, slot) in self.history.iter().enumerate() {
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(4), Constraint::Min(0)])
                .split(rows[i]);

            match slot {
                None => {
                    let block = Block::default().style(Style::default().bg(Color::from_u32(0)));
                    frame.render_widget(block, cols[0]);
                    frame.render_widget(Paragraph::new("-----"), cols[1]);
                }
                Some(guess) => {
                    // Color block
                    let color = u32::from_str_radix(&guess.input, 16).unwrap();
                    let block = Block::default().style(Style::default().bg(Color::from_u32(color)));
                    frame.render_widget(block, cols[0]);

                    // feedbacck spans
                    let spans: Vec<Span> = guess
                        .input
                        .chars()
                        .enumerate()
                        .map(|(i, c)| {
                            let color = match guess.feedback[i] {
                                DigitFeedback::Correct => Color::Green,
                                DigitFeedback::Close => Color::Yellow,
                                DigitFeedback::Wrong => Color::Red,
                            };
                            Span::styled(c.to_string(), Style::default().fg(color))
                        })
                        .collect();

                    frame.render_widget(Paragraph::new(Line::from(spans)), cols[1]);
                }
            }
        }
    }

    fn draw_input(&self, frame: &mut Frame, area: Rect) {
        let text = format!("# {}_", self.current_attempt);
        let paragraph =
            Paragraph::new(text).block(Block::default().borders(Borders::ALL).title(" Attempt "));
        frame.render_widget(paragraph, area);
    }

    fn draw_game_over(&self, frame: &mut Frame, area: Rect) {
        let (title, message, color) = match self.state {
            GameState::Won => (
                " 🎉 YOU WIN! 🎉 ",
                format!(
                    "The color was #{}\n\nPress R to play again\nPress ESC to quit",
                    self.color
                ),
                Color::Green,
            ),
            GameState::Lost => (
                " 💀 GAME OVER 💀 ",
                format!(
                    "The color was #{}\n\nPress R to play again\nPress ESC to quit",
                    self.color
                ),
                Color::Red,
            ),
            _ => return,
        };

        // Show the hidden color
        let color_val = u32::from_str_radix(&self.color, 16).unwrap();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Color block
                Constraint::Min(0),    // Message
            ])
            .split(area);

        // Color block block
        let color_block = Block::default().style(Style::default().bg(Color::from_u32(color_val)));
        frame.render_widget(color_block, chunks[0]);

        // Message block
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .title(title)
            .title_alignment(Alignment::Center);

        let paragraph = Paragraph::new(message)
            .block(block)
            .alignment(Alignment::Center);

        frame.render_widget(Clear, area);
        frame.render_widget(paragraph, chunks[1]);
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(vertical[1])[1]
    }
}
