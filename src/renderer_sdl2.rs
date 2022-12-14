use crate::{
    Item,
    ColorPalette,
    RendererEvent,
    Renderer,
};
use std::{
    sync::{mpsc, Mutex},
    collections::HashMap,
};
use sdl2::{
    render::{self, Texture},
    rect::Rect,
    event,
    ttf,
    video,
};
use anyhow::Result;

pub fn new() -> Result<SdlRenderer> {
    SdlRenderer::new()
}

pub struct SdlRenderer {
    sdl2: sdl2::Sdl,
    video: sdl2::VideoSubsystem,
    canvas: render::Canvas<video::Window>,
    ttf: ttf::Sdl2TtfContext,
    text_creator: render::TextureCreator<video::WindowContext>,
    text_map: HashMap<String, Texture>,
    rx_mutex: Mutex<Option<mpsc::Receiver<RendererEvent>>>,
    event_watch: event::EventWatch<'static, RendererEventWatch>,
    pump: sdl2::EventPump,
    fontsize: u16,
    font_height: u32,
}

struct RendererEventWatch {
    chan: mpsc::Sender<RendererEvent>,
}

impl sdl2::event::EventWatchCallback for RendererEventWatch {
    fn callback(&mut self, ev: sdl2::event::Event) {
        use sdl2::{
            event::{
                Event,
                WindowEvent,
            },
            keyboard::Keycode,
        };
        use crate::HidEvent;

        let new_ev = match ev {
            Event::Quit{..} => RendererEvent::WindowClosed,
            Event::Window{win_event, ..} => match win_event {
                WindowEvent::Resized(..) | WindowEvent::SizeChanged(..) => RendererEvent::Refresh,
                WindowEvent::Close => RendererEvent::WindowClosed,
                _ => return,
            },
            Event::KeyDown{keycode: Some(key), ..} => match key {
                Keycode::X      => RendererEvent::Hid(HidEvent::NextTab),
                Keycode::Z      => RendererEvent::Hid(HidEvent::PreviousTab),
                Keycode::Left   => RendererEvent::Hid(HidEvent::Left),
                Keycode::Right  => RendererEvent::Hid(HidEvent::Right),
                Keycode::Up     => RendererEvent::Hid(HidEvent::Up),
                Keycode::Down   => RendererEvent::Hid(HidEvent::Down),
                Keycode::Return => RendererEvent::Hid(HidEvent::ButtonPress),
                _ => return,
            }
            _ => return,
        };

        self.chan.send(new_ev).expect("Failed to send RendererEvent");
    }
}

impl SdlRenderer {
    fn new() -> Result<Self> {
        sdl2::hint::set("SDL_VIDEO_EGL_ALLOW_TRANSPARENCY", "1");

        let sdl2 = sdl2::init().expect("Failed to initialize SDL2");
        let video = sdl2.video().expect("Failed to initalize SDL2 video subsystem");
        let ev = sdl2.event().expect("Failed to initialize SDL2 event subsystem");

        let ttf = ttf::init()?;

        let window = video.window("SGui window", 480, 320)
            .resizable()
            .build()?;
        let mut canvas = window.into_canvas()
            .present_vsync()
            .build()?;
        let text_creator = canvas.texture_creator();

        canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, 255));
        canvas.clear();
        canvas.present();

        let (tx, rx) = mpsc::channel();
        let event_watch = ev.add_event_watch(RendererEventWatch{chan: tx});
        let pump = sdl2.event_pump().expect("Failed to get SDL2 event pump");

        //let font_rwops = rwops::RWops::from_file("/usr/share/fonts/liberation/LiberationSans-Regular.ttf", "r");
        let font = ttf.load_font("/usr/share/fonts/liberation/LiberationSans-Regular.ttf", 28).expect("Failed to load font");
        let font_height = font.height() as u32;
        drop(font);

        Ok(Self {
            sdl2,
            video,
            canvas,
            ttf,
            text_creator,
            text_map: HashMap::new(),
            rx_mutex: Mutex::new(Some(rx)),
            event_watch,
            pump,
            fontsize: 28,
            font_height,
        })
    }
    fn ensure_text_is_rendered(&mut self, input: &str, color: (u8, u8, u8), size: u16) -> Result<()> {
        if self.text_map.get_mut(&input.to_string()).is_some() {
            return Ok(());
        };

        let font = self.ttf.load_font("/usr/share/fonts/liberation/LiberationSans-Regular.ttf", size).expect("Failed to load font");
        let surface = font.render(input).blended(color)?;
        let texture = self.text_creator.create_texture_from_surface(&surface)?;

        self.text_map.insert(input.to_string(), texture);
        Ok(())
    }
}

impl Renderer for SdlRenderer {
    fn tick(&mut self) {
        self.pump.pump_events();
    }
    fn get_event(&self) -> Option<mpsc::Receiver<RendererEvent>> {
        self.rx_mutex.lock().unwrap().take()
    }
    fn draw_tab_header(&mut self, names: &[&str], colors: &ColorPalette) -> Result<()> {
        let width = self.canvas.viewport().width();
        self.canvas.set_viewport(Rect::new(0 as i32, 0 as i32, width, self.font_height));
        self.canvas.set_draw_color(colors.tab_bg.as_tuple());
        self.canvas.clear();

        let mut offset = 0;

        for (i, name) in names.iter().enumerate() {
            self.ensure_text_is_rendered(name, colors.tab_text.as_tuple(), self.fontsize)?;
            let texture = self.text_map.get_mut(&name.to_string()).unwrap();

            if i == 0 {
                texture.set_color_mod(colors.tab_accent.r, colors.tab_accent.g, colors.tab_accent.b);
            }

            let query = texture.query();

            // outline
            let outline_rect = Rect::new(offset, 0, query.width + 1, self.font_height);
            self.canvas.set_draw_color(colors.tab_outline.as_tuple());
            self.canvas.draw_rect(outline_rect)
                .expect("Failed to draw tab outline");

            // tab name
            let text_rect = Rect::new(offset, 0, query.width, query.height);
            self.canvas.copy(&texture, None, text_rect)
                .expect("Failed to draw tab header text");

            offset += query.width as i32;
        }

        let full_outline = Rect::new(0, 0, width, self.font_height);
        self.canvas.set_draw_color(colors.tab_outline.as_tuple());
        self.canvas.draw_rect(full_outline)
            .expect("Failed to draw tab outline");

        self.canvas.present();

        self.canvas.set_viewport(None);
        Ok(())
    }
    fn draw_items(&mut self, items: &Vec<Vec<Item>>, colors: &ColorPalette, selected_item_idx: (usize, usize)) -> Result<()> {
        let old_viewport = self.canvas.viewport();
        self.canvas.set_viewport(Rect::new(0 as i32, self.font_height as i32, old_viewport.width(), old_viewport.height() - self.font_height));
        self.canvas.set_draw_color(colors.item_bg.as_tuple());
        self.canvas.draw_rect(self.canvas.viewport())
            .expect("Failed to clear area on which items will be drawn");
        let font_height = self.font_height;

        for (y_offset, line) in items.iter().enumerate().map(|(i, v)| (i * font_height as usize, v)) {
            if line.len() == 0 {
                continue;
            }
            let x_step = old_viewport.width() as usize / line.len();

            for (x_offset, item) in line.iter().enumerate().map(|(i, v)| (i * x_step, v)) {
                match item {
                    Item::Text(text) | Item::StatelessButton(text) => {                
                        self.ensure_text_is_rendered(text, colors.item_text.as_tuple(), self.fontsize)?;
                        let texture = self.text_map.get_mut(&text.to_string()).unwrap();
                        let query = texture.query();
                        let text_rect = Rect::new(x_offset as i32, y_offset as i32, query.width, query.height);

                        self.canvas.copy(&texture, None, text_rect)
                            .expect("Failed to draw tab header text");
                    },
                    Item::StatefulButton(text, state) => {
                        self.ensure_text_is_rendered(text, colors.item_text.as_tuple(), self.fontsize)?;
                        let texture = self.text_map.get_mut(&text.to_string()).unwrap();
                        if *state {
                            texture.set_color_mod(colors.item_accent.r, colors.item_accent.g, colors.item_accent.b);
                        }
                        let query = texture.query();
                        let text_rect = Rect::new(x_offset as i32, y_offset as i32, query.width, query.height);

                        self.canvas.copy(&texture, None, text_rect)
                            .expect("Failed to draw tab header text");
                    },
                }
            }
        }

        self.canvas.present();
        self.canvas.set_viewport(None);
        Ok(())
    }
}
