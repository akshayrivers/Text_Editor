use super::{
    terminal::{Size, Terminal},
    Message,
};
pub struct MessageBar {
    current_msg: Message,
    needs_redraw: bool,
    margin_bottom: usize,
    width: usize,
    position_y: usize,
    is_visible: bool,
}

impl MessageBar {
    pub fn new(margin_bottom: usize) -> Self {
        let size = Terminal::size().unwrap_or_default();
        let mut message_bar = Self {
            current_msg: Message::default(),
            needs_redraw: true,
            margin_bottom,
            width: size.width,
            position_y: 0,
            is_visible: false,
        };
        message_bar.resize(size);
        message_bar
    }

    pub fn resize(&mut self, size: Size) {
        self.width = size.width;
        let mut position_y = 0;
        let mut is_visible = false;
        if let Some(result) = size
            .height
            .checked_sub(self.margin_bottom)
            .and_then(|result| result.checked_sub(1))
        {
            position_y = result;
            is_visible = true;
        }
        self.position_y = position_y;
        self.is_visible = is_visible;
        self.needs_redraw = true;
    }
    pub fn render(&mut self) {
        if !self.needs_redraw || !self.is_visible {
            return;
        }
        if let Ok(size) = Terminal::size() {
            if self.current_msg.msg.is_empty() {
                self.current_msg.default_message();
            }

            let msg = format!("{}", self.current_msg.msg);
            let to_print = if msg.len() <= size.width {
                msg
            } else {
                String::new()
            };
            let result = Terminal::print_row(self.position_y, &to_print);
            debug_assert!(result.is_ok(), "Failed to render message bar");
            self.needs_redraw = false;
        }
    }
}
