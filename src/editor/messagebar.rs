use super::{
    terminal::{Size, Terminal},
    uicomponent::UIComponent,
};
use std::io::Error;
#[derive(Default)]
pub struct MessageBar {
    current_msg: String,
    needs_redraw: bool,
}

impl MessageBar {
    pub fn update_message(&mut self, new_message: String) {
        if new_message != self.current_msg {
            self.current_msg = new_message;
            self.mark_redraw(true);
        }
    }
}

impl UIComponent for MessageBar {
    fn mark_redraw(&mut self, value: bool) {
        self.needs_redraw = value;
    }
    fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }
    fn set_size(&mut self, _: Size) {}
    fn draw(&mut self, origin: usize) -> Result<(), Error> {
        Terminal::print_row(origin, &self.current_msg)
    }
}
