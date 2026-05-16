use super::{Position, Size};
#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct Rect {
    pub position: Position,
    pub size: Size,
}
