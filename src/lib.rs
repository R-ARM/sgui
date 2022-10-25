pub mod layout;
//pub mod renderer_sdl2;
pub mod renderer_crossterm;

use layout::Item;
use anyhow::Result;
use std::{
    sync::mpsc,
    thread,
    time::Duration,
};

pub struct Color{r: u8, g: u8, b: u8}
#[allow(dead_code)]
pub struct ColorPalette {
    tab_outline: Color,
    tab_text: Color,
    tab_bg: Color,
    tab_accent: Color,

    item_outline: Color,
    item_text: Color,
    item_bg: Color,
    item_accent: Color,
}

impl Color {
    fn as_crossterm_color(&self) -> crossterm::style::Color {
        (self.r, self.g, self.b).into()
    }
}

impl ColorPalette {
    fn default() -> Self {
        Self {
            tab_outline: Color{r: 255, g: 255, b: 255},
            tab_text: Color{r: 255, g: 255, b: 255},
            tab_bg: Color{r: 0, g: 0, b: 0},
            tab_accent: Color{r: 255, g: 0, b: 0},
            
            item_outline: Color{r: 255, g: 0, b: 0},
            item_text: Color{r: 255, g: 255, b: 255},
            item_bg: Color{r: 0, g: 0, b: 0},
            item_accent: Color{r: 255, g: 0, b: 0},
        }
    }
}


pub enum GuiEvent {
    ItemSelected(String),
    StatefulButtonChange(String, bool),
    StatelessButtonPress(String),
    TabChanged(String)
}
pub enum HidEvent {
    Up,
    Down,
    Left,
    Right,
    NextTab,
    PreviousTab,
    ButtonPress,
}

pub trait Renderer {
    fn draw_tab_header(&mut self, names: &[&str], colors: &ColorPalette, selected_tab_idx: u64) -> Result<()>;
    fn draw_items(&mut self, items: &Vec<Vec<layout::Item>>, colors: &ColorPalette, selected_item_idx: (usize, usize)) -> Result<()>;
    fn get_input(&self) -> Option<mpsc::Receiver<HidEvent>>;
}

pub struct Gui {
    renderer: Box<dyn Renderer>,
    layout: layout::Layout,
    colors: ColorPalette,
    hid_rx: Option<mpsc::Receiver<HidEvent>>,
    tab_pos: i32,
    item_pos: (usize, usize),
}

impl Gui {
    pub fn get_ev(&mut self) -> GuiEvent {
        loop {
            let mut ret = None;
            let mut redraw_items = false;
            let mut redraw_tabs = false;

            // handle events made by renderer
            let mut tab_chg = 0;
            let mut item_column_chg: i32 = 0;
            let mut item_row_chg: i32 = 0;
            let mut activate_selection = false;

            if let Some(rx) = &self.hid_rx {
                if let Ok(ev) = rx.recv_timeout(Duration::from_millis(50)) {
                    match ev {
                        HidEvent::NextTab => tab_chg = 1,
                        HidEvent::PreviousTab => tab_chg = -1,
                        HidEvent::Up => item_row_chg = -1,
                        HidEvent::Down => item_row_chg = 1,
                        HidEvent::Left => item_column_chg = -1,
                        HidEvent::Right => item_column_chg = 1,
                        HidEvent::ButtonPress => activate_selection = true,
                    }
                }
            }

            if activate_selection {
                let (row, col) = self.item_pos;
                if let Some(tab) = self.layout.tab_mut(self.tab_pos as usize) {
                    if let Some(row) = tab.items_mut().get_mut(row) {
                        if let Some(item) = row.get_mut(col) {
                            match item {
                                &mut Item::StatefulButton(ref text, ref mut state) => {
                                    *state = !*state;
                                    redraw_items = true;
                                    ret = Some(GuiEvent::StatefulButtonChange(text.to_string(), *state));
                                },
                                Item::StatelessButton(text) => {
                                    ret = Some(GuiEvent::StatelessButtonPress(text.to_string()));
                                },
                                _ => (),
                            }
                        }
                    }
                }
            }

            // change tab if we need to, and refresh everything if we changed a tab
            if tab_chg != 0 {
                self.tab_pos = (self.tab_pos + tab_chg).clamp(0, self.layout.tab_count());
                self.item_pos = (0, 0);

                redraw_tabs = true;
                redraw_items = true;

                match tab_chg {
                    1  => ret = Some(GuiEvent::TabChanged("todo".to_string())),
                    -1 => ret = Some(GuiEvent::TabChanged("todo".to_string())),
                    _ => (),
                }
            }

            if item_row_chg != 0 {
                if let Some(curtab) = self.layout.tab(self.tab_pos as usize) {
                    let (cur_row, cur_column) = self.item_pos;
                    
                    let max_row = (curtab.items().len() as i32 - 1).clamp(0, 10000);
                    let new_cur_row = (cur_row as i32 + item_row_chg).clamp(0, max_row) as usize;

                    // we have to check because we're moving selection to another row
                    if let Some(row) = curtab.items().get(new_cur_row) {
                        if let Some(_item) = row.get(cur_column) {
                            self.item_pos = (new_cur_row, cur_column);
                            redraw_items = true;
                        }
                    }
                }
            }

            if item_column_chg != 0 {
                if let Some(curtab) = self.layout.tab(self.tab_pos as usize) {
                    let (cur_row, cur_column) = self.item_pos;
                    let max_column;
                    let new_cur_column;

                    if let Some(row) = curtab.items().get(cur_row) {
                        max_column = (row.len() as i32 - 1).clamp(0, 10000);
                        new_cur_column = (cur_column as i32 + item_column_chg).clamp(0, max_column) as usize;
                    } else {
                        new_cur_column = 0;
                    }

                    self.item_pos = (cur_row, new_cur_column);
                    redraw_items = true;
                }
            }

            if redraw_tabs {
                self.renderer.draw_tab_header(&self.layout.tab_names(), &self.colors, self.tab_pos as u64)
                    .expect("Failed to draw tab header");
            }

            if redraw_items {
                if let Some(curtab) = self.layout.tab(self.tab_pos as usize) {
                    self.renderer.draw_items(curtab.items(), &self.colors, self.item_pos)
                        .expect("Failed to draw items");
                }
            }

            if let Some(return_this) = ret {
                return return_this;
            }

            thread::sleep(Duration::from_millis(50));
        }
    }
    pub fn new(layout: layout::Layout) -> Gui {
        let colors = ColorPalette::default();
        let mut renderer = autopick_renderer();
        renderer.draw_tab_header(&layout.tab_names(), &colors, 0).unwrap();
        renderer.draw_items(&layout.tab(0).unwrap().items(), &colors, (0, 0)).unwrap();
        let hid_rx = renderer.get_input();

        Gui {
            layout,
            renderer,
            colors,
            hid_rx,
            tab_pos: 0,
            item_pos: (0, 0),
        }
    }
}

fn autopick_renderer() -> Box<dyn Renderer> {
    Box::new(renderer_crossterm::new().unwrap())
    //Box::new(renderer_sdl2::new())
}
