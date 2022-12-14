use sgui::layout::Layout;
use sgui::Gui;
use sgui::GuiEvent;

fn main() {
    let layout = Layout::builder()
        .tab("Example tab")
            .line()
                .text("Sample Text")
                .text("More Text")
            .line()
                .button_stateful("I'm a button!", true, 1)
        .tab("Another tab, empty")
        .tab("I AM A TAB")
            .line()
                .button_stateful("baton", false, 2)
                .button_stateless("i don't have a state lol", 123)
        .build();

    let mut gui = Gui::new(layout);
    let state = loop {
        let ev = gui.get_ev();
        if ev == GuiEvent::Quit {
            let state = gui.exit_dumping_state();
            break state;
        }
    };

    println!("{:#?}", state);
}
