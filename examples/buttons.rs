use sgui::layout::Layout;
use sgui::Gui;

fn main() {
    let layout = Layout::builder()
        .tab("Example tab")
            .line()
                .text("Sample Text")
                .text("More Text")
            .line()
                .button_stateful("I'm a button!", true)
        .tab("Another tab, empty")
        .tab("I AM A TAB")
            .line()
                .button_stateful("baton", false)
                .button_stateless("i don't have a state lol")
        .build();
    let mut gui = Gui::new(layout);
    loop {
        gui.get_ev();
    }
}
