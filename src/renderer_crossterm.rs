use crate::{Renderer, ColorPalette, HidEvent, RendererEvent, layout::Item};
use anyhow::Result;
use std::{
    io::{self, Write},
    collections::HashSet,
    sync::mpsc,
    thread,
};
use crossterm::{
    ExecutableCommand,
    QueueableCommand,
    cursor,
    event::{
        self,
        KeyCode,
        Event,
        KeyEventKind,
    },
    terminal,
    style,
};

pub fn new() -> Result<CrosstermRenderer> {
    let mut out = io::stdout();
    out.execute(terminal::EnterAlternateScreen)?;
    out.execute(cursor::Hide)?;
    terminal::enable_raw_mode()?;

    Ok(CrosstermRenderer {
        out: io::stdout(),
    })
}

impl Drop for CrosstermRenderer {
    fn drop(&mut self) {
        self.out.execute(cursor::Show).unwrap();
        self.out.execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
}

pub struct CrosstermRenderer {
    out: io::Stdout,
}

fn handle_events(tx: mpsc::Sender<RendererEvent>) {
    loop {
        match event::read() {
            Ok(ev) => {
                if match ev {
                    Event::Key(key) => {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        let ev = match key.code {
                            KeyCode::Up => HidEvent::Up,
                            KeyCode::Down => HidEvent::Down,
                            KeyCode::Left => HidEvent::Left,
                            KeyCode::Right => HidEvent::Right,
                            KeyCode::Enter => HidEvent::ButtonPress,
                            KeyCode::Tab => HidEvent::NextTab,
                            KeyCode::BackTab => HidEvent::PreviousTab,
                            KeyCode::Esc => HidEvent::Quit,
                            _ => continue,
                        };
                        tx.send(RendererEvent::Hid(ev))
                    },
                    Event::Resize(_, _) => tx.send(RendererEvent::Refresh),
                    _ => continue,
                }.is_err() {
                    break;
                }
            },
            Err(_) => break,
        }
    }
}

impl Renderer for CrosstermRenderer {
    fn get_event(&self) -> Option<mpsc::Receiver<RendererEvent>> {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || handle_events(tx));
        Some(rx)
    }
    fn draw_tab_header(&mut self, names: &[&str], colors: &ColorPalette) -> Result<()> {
        let (columns, _) = terminal::size()?;

        // set up places where to put | characters
        let mut vert_x = HashSet::new();
        let mut tmp = 0;
        for name in names.iter() {
            vert_x.insert(tmp);
            tmp += name.len();
            tmp += 1;
        }
        vert_x.insert(tmp);
        vert_x.insert(columns as usize - 1);


        // draw vertical line above and below
        self.out.queue(terminal::Clear(terminal::ClearType::All))?;
        self.out.queue(cursor::MoveTo(0, 0))?;
        self.out.queue(style::SetForegroundColor(colors.tab_outline.as_crossterm_color()))?;
        for x in 0..columns {
            match x { // note to Maya in future: the order really is important
                0 => self.out.queue(style::Print("┌"))?,
                _ if x == columns-1 => self.out.queue(style::Print("┐"))?,
                _ if vert_x.contains(&(x as usize)) => self.out.queue(style::Print("┬"))?,
                _ => self.out.queue(style::Print("─"))?,
            };
        }
        self.out.queue(cursor::MoveTo(0, 2))?;
        for x in 0..columns {
            match x {
                0 => self.out.queue(style::Print("└"))?,
                _ if x == columns-1 => self.out.queue(style::Print("┘"))?,
                _ if vert_x.contains(&(x as usize)) => self.out.queue(style::Print("┴"))?,
                _ => self.out.queue(style::Print("─"))?,
            };
        }

        // draw tab names
        self.out.queue(cursor::MoveTo(0, 1))?;
        self.out.queue(style::SetForegroundColor(colors.tab_text.as_crossterm_color()))?;
        for (i, name) in names.iter().map(|v| format!(" {}", v)).enumerate() {
            if i == 0 {
                self.out.queue(style::SetForegroundColor(colors.tab_accent.as_crossterm_color()))?;
                self.out.queue(style::Print(&name))?;
                self.out.queue(style::SetForegroundColor(colors.tab_text.as_crossterm_color()))?;
            } else {
                self.out.queue(style::Print(&name))?;
            }
        }

        // draw | between tab names
        self.out.queue(style::SetForegroundColor(colors.tab_outline.as_crossterm_color()))?;
        for x in vert_x.into_iter() {
            self.out.queue(cursor::MoveTo(x.try_into().unwrap(), 1))?;
            self.out.queue(style::Print("│"))?;
        }

        self.out.flush()?;
        Ok(())
    }
    fn draw_items(&mut self, items: &Vec<Vec<Item>>, colors: &ColorPalette, selected_item_idx: (usize, usize)) -> Result<()> {
        let (columns, rows) = terminal::size()?;
        let (selected_item_i, selected_item_j) = selected_item_idx;
        // TODO: scrolling
        for (cur_line, line) in items.iter().enumerate() {
            if cur_line > rows as usize {
                break;
            }
            let items_num = line.len() as u16;
            let item_x_offset = columns/items_num;

            for (j, item) in line.iter().enumerate() {
                self.out.queue(cursor::MoveTo(item_x_offset * j as u16, 3 + cur_line as u16))?;
                if cur_line == selected_item_i && j == selected_item_j {
                    self.out.queue(style::SetForegroundColor(colors.item_accent.as_crossterm_color()))?;
                } else {
                    self.out.queue(style::SetForegroundColor(colors.item_text.as_crossterm_color()))?;
                }
                match item {
                    Item::Text(text) | Item::StatelessButton(text, _) => {
                        self.out.queue(style::Print(&text))?;
                    },
                    Item::StatefulButton(text, state, _) => {
                        if *state {
                            self.out.queue(style::Print("[ ] "))?;
                        } else {
                            self.out.queue(style::Print("[X] "))?;
                        }
                        self.out.queue(style::Print(&text))?;
                    },
                };
            }
        }
        self.out.flush()?;
        Ok(())
    }
    fn tick(&mut self) {}
}
