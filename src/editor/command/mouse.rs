use crate::prelude::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[derive(Clone, Copy, Debug)]
pub enum MouseCommand {
    LeftClick(Position),
    LeftDrag(Position),
    LeftRelease(Position),
    ScrollUp(Position),
    ScrollDown(Position),
}

impl TryFrom<MouseEvent> for MouseCommand {
    type Error = String;

    fn try_from(event: MouseEvent) -> Result<Self, Self::Error> {
        let position = Position {
            row: event.row as usize,
            col: event.column as usize,
        };

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => Ok(Self::LeftClick(position)),
            MouseEventKind::Drag(MouseButton::Left) => Ok(Self::LeftDrag(position)),
            MouseEventKind::Up(MouseButton::Left) => Ok(Self::LeftRelease(position)),
            MouseEventKind::ScrollUp => Ok(Self::ScrollUp(position)),
            MouseEventKind::ScrollDown => Ok(Self::ScrollDown(position)),
            _ => Err("Mouse event not supported".to_string()),
        }
    }
}
