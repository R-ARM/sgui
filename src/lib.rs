pub mod layout;
#[cfg(feature = "sdl2")]
pub mod renderer_sdl2;
pub mod renderer_crossterm;

use layout::Item;
use anyhow::Result;
use ez_input::RinputerHandle;
use std::{
    sync::mpsc,
    thread,
    time::Duration,
};

#[derive(Debug)]
pub struct Color{r: u8, g: u8, b: u8}
#[derive(Debug)]
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
    fn as_tuple(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
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

#[derive(Eq, PartialEq, Debug)]
pub enum GuiEvent {
    ItemSelected(String),
    StatefulButtonChange(String, bool, u128),
    StatelessButtonPress(String, u128),
    TabChanged(String),
    Quit,
}

#[derive(Eq, PartialEq, Debug)]
pub enum HidEvent {
    Up,
    Down,
    Left,
    Right,
    NextTab,
    PreviousTab,
    ButtonPress,
    Quit,
}

#[derive(Eq, PartialEq, Debug)]
pub enum RendererEvent {
    Refresh,
    WindowClosed,
    Hid(HidEvent),
}

pub trait Renderer {
    fn draw_tab_header(&mut self, names: &[&str], colors: &ColorPalette) -> Result<()>;
    fn draw_items(&mut self, items: &Vec<Vec<layout::Item>>, colors: &ColorPalette, selected_item_idx: (usize, usize)) -> Result<()>;
    fn get_event(&self) -> Option<mpsc::Receiver<RendererEvent>>;
    fn tick(&mut self);
}

pub struct Gui {
    renderer: Box<dyn Renderer>,
    layout: layout::Layout,
    colors: ColorPalette,
    hid_rx: Option<mpsc::Receiver<HidEvent>>,
    renderer_rx: Option<mpsc::Receiver<RendererEvent>>,
    tab_pos: i32,
    item_pos: (usize, usize),
    ignore_hid: bool,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct GuiState {
    layout: layout::Layout,
    tab_pos: i32,
    item_pos: (usize, usize),
}

impl Gui {
    pub fn exit_dumping_state(self) -> GuiState {
        GuiState {
            layout: self.layout,
            tab_pos: self.tab_pos,
            item_pos: self.item_pos,
        }
    }
    pub fn set_ignore_hid(&mut self, val: bool) {
        self.ignore_hid = val;
    }
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
            let mut hid_ev = None;

            if let Some(rx) = &self.hid_rx {
                hid_ev = rx.recv_timeout(Duration::from_millis(10)).ok();
            }

            if hid_ev.is_none() {
                if let Some(rx) = &self.renderer_rx {
                    if let Ok(r_ev) = rx.recv_timeout(Duration::from_millis(10)){
                        if !self.ignore_hid {
                            match r_ev {
                                RendererEvent::Refresh => {
                                    redraw_items = true;
                                    redraw_tabs = true;
                                },
                                RendererEvent::WindowClosed => {
                                    ret = Some(GuiEvent::Quit);
                                },
                                RendererEvent::Hid(ev) => {
                                    hid_ev = Some(ev);
                                }
                            }
                        }
                    }
                }
            }

            if let Some(hid_ev) = hid_ev {
                if !self.ignore_hid {
                    match hid_ev {
                        HidEvent::NextTab => tab_chg = 1,
                        HidEvent::PreviousTab => tab_chg = -1,
                        HidEvent::Up => item_row_chg = -1,
                        HidEvent::Down => item_row_chg = 1,
                        HidEvent::Left => item_column_chg = -1,
                        HidEvent::Right => item_column_chg = 1,
                        HidEvent::ButtonPress => activate_selection = true,
                        HidEvent::Quit => ret = Some(GuiEvent::Quit),
                    }
                }
            }

            if activate_selection {
                let (row, col) = self.item_pos;
                if let Some(tab) = self.layout.tab_mut(self.tab_pos as usize) {
                    if let Some(row) = tab.items_mut().get_mut(row) {
                        if let Some(item) = row.get_mut(col) {
                            match item {
                                &mut Item::StatefulButton(ref text, ref mut state, ref id) => {
                                    *state = !*state;
                                    redraw_items = true;
                                    ret = Some(GuiEvent::StatefulButtonChange(text.to_string(), *state, *id));
                                },
                                Item::StatelessButton(text, id) => {
                                    ret = Some(GuiEvent::StatelessButtonPress(text.to_string(), *id));
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
                self.renderer.draw_tab_header(&self.layout.tab_names().into_iter().skip(self.tab_pos as usize).collect::<Vec<&str>>(), &self.colors)
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

            self.renderer.tick();
        }
    }
    pub fn new(layout: layout::Layout) -> Gui {
        let colors = ColorPalette::default();
        let mut renderer = autopick_renderer();
        renderer.draw_tab_header(&layout.tab_names(), &colors).unwrap();
        renderer.draw_items(&layout.tab(0).unwrap().items(), &colors, (0, 0)).unwrap();
        let renderer_rx = renderer.get_event();

        let hid_rx = autopick_input();

        Gui {
            layout,
            renderer,
            colors,
            hid_rx,
            renderer_rx,
            tab_pos: 0,
            item_pos: (0, 0),
            ignore_hid: false,
        }
    }
}

fn autopick_input() -> Option<mpsc::Receiver<HidEvent>> {
    let mut handle = RinputerHandle::open()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        loop {
            use ez_input::EzEvent;
            let Some(event) = handle.get_event_blocking() else {continue};
            let ev = match event {
                EzEvent::DirectionUp => HidEvent::Up,
                EzEvent::DirectionDown => HidEvent::Down,
                EzEvent::DirectionLeft => HidEvent::Left,
                EzEvent::DirectionRight => HidEvent::Right,
                EzEvent::South(true) => HidEvent::ButtonPress,
                EzEvent::R(true) => HidEvent::NextTab,
                EzEvent::L(true) => HidEvent::PreviousTab,
                _ => continue,
            };
            if tx.send(ev).is_err() {
                break;
            };
        }
    });

    Some(rx)
}

fn autopick_renderer() -> Box<dyn Renderer> {
    #[cfg(feature = "sdl2")]
    if let Ok(sdl) = renderer_sdl2::new() {
        return Box::new(sdl);
    }

    Box::new(renderer_crossterm::new().unwrap())
}
