use crate::prelude::*;
use std::io::Error;

pub enum LayoutNode {
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
        rect: Rect,
    },
    Leaf {
        pane_id: usize,
        rect: Rect,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}
pub struct LayoutTree {
    root: LayoutNode,
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new(0, Rect::default())
    }
}
impl LayoutTree {
    // construction
    pub fn new(initial_pane_id: usize, rect: Rect) -> Self {
        Self {
            root: LayoutNode::Leaf {
                pane_id: initial_pane_id,
                rect,
            },
        }
    }

    // layout computation
    pub fn compute_layout(&mut self, rect: Rect) {
        Self::compute_node_layout(&mut self.root, rect);
    }

    fn compute_node_layout(node: &mut LayoutNode, rect: Rect) {
        match node {
            LayoutNode::Leaf {
                rect: node_rect, ..
            } => {
                *node_rect = rect;
            }

            LayoutNode::Split {
                direction,
                ratio,
                first,
                second,
                rect: node_rect,
            } => {
                *node_rect = rect;

                match direction {
                    SplitDirection::Vertical => {
                        let first_width = (rect.size.width as f32 * *ratio) as usize;
                        let second_width = rect.size.width.saturating_sub(first_width);

                        let first_rect = Rect {
                            position: rect.position,
                            size: Size {
                                width: first_width,
                                height: rect.size.height,
                            },
                        };

                        let second_rect = Rect {
                            position: Position {
                                row: rect.position.row,
                                col: rect.position.col + first_width,
                            },
                            size: Size {
                                width: second_width,
                                height: rect.size.height,
                            },
                        };

                        Self::compute_node_layout(first, first_rect);
                        Self::compute_node_layout(second, second_rect);
                    }

                    SplitDirection::Horizontal => {
                        let first_height = (rect.size.height as f32 * *ratio) as usize;
                        let second_height = rect.size.height.saturating_sub(first_height);

                        let first_rect = Rect {
                            position: rect.position,
                            size: Size {
                                width: rect.size.width,
                                height: first_height,
                            },
                        };

                        let second_rect = Rect {
                            position: Position {
                                row: rect.position.row + first_height,
                                col: rect.position.col,
                            },
                            size: Size {
                                width: rect.size.width,
                                height: second_height,
                            },
                        };

                        Self::compute_node_layout(first, first_rect);
                        Self::compute_node_layout(second, second_rect);
                    }
                }
            }
        }
    }

    // mutation
    pub fn split_pane(
        &mut self,
        target_pane_id: usize,
        new_pane_id: usize,
        direction: SplitDirection,
        ratio: f32,
    ) -> Result<(), Error> {
        Self::split_node(
            &mut self.root,
            target_pane_id,
            new_pane_id,
            direction,
            ratio,
        )
    }

    fn split_node(
        node: &mut LayoutNode,
        target_pane_id: usize,
        new_pane_id: usize,
        direction: SplitDirection,
        ratio: f32,
    ) -> Result<(), Error> {
        match node {
            LayoutNode::Leaf { pane_id, rect } => {
                if *pane_id == target_pane_id {
                    let old_rect = *rect;

                    let old_leaf = LayoutNode::Leaf {
                        pane_id: *pane_id,
                        rect: old_rect,
                    };

                    let new_leaf = LayoutNode::Leaf {
                        pane_id: new_pane_id,
                        rect: old_rect,
                    };

                    *node = LayoutNode::Split {
                        direction,
                        ratio,
                        first: Box::new(old_leaf),
                        second: Box::new(new_leaf),
                        rect: old_rect,
                    };

                    Ok(())
                } else {
                    Err(Error::other("Pane not found"))
                }
            }

            LayoutNode::Split { first, second, .. } => {
                if Self::split_node(first, target_pane_id, new_pane_id, direction, ratio).is_ok() {
                    return Ok(());
                }

                Self::split_node(second, target_pane_id, new_pane_id, direction, ratio)
            }
        }
    }
    // when the child node is removed we need to adjust the tree accordingly too
    pub fn remove_node(&mut self, id: usize) -> Result<(), Error> {
        // if there is only one pane left we do not allow closing
        if let LayoutNode::Leaf { pane_id, .. } = self.root {
            if pane_id == id {
                return Err(Error::other("Cannot remove the last remaining pane"));
            }
        }

        let old_root = std::mem::replace(
            &mut self.root,
            LayoutNode::Leaf {
                pane_id: 0,
                rect: Rect::default(),
            },
        );
        match self.remove_node_recursive(old_root, id) {
            Ok(new_root) => {
                self.root = new_root;
                Ok(())
            }
            //still just in case
            Err(e) if e.to_string() == "DELETED" => {
                Err(Error::other("Unexpected deletion of root"))
            }
            Err(e) => Err(e),
        }
    }

    fn remove_node_recursive(
        &self,
        node: LayoutNode,
        target_id: usize,
    ) -> Result<LayoutNode, Error> {
        match node {
            LayoutNode::Leaf { pane_id, rect } => {
                if pane_id == target_id {
                    return Err(Error::other("DELETED"));
                }
                Ok(LayoutNode::Leaf { pane_id, rect })
            }
            LayoutNode::Split {
                direction,
                ratio,
                first,
                second,
                rect,
            } => {
                match self.remove_node_recursive(*first, target_id) {
                    Err(e) if e.to_string() == "DELETED" => Ok(*second),
                    Ok(new_first) => {
                        match self.remove_node_recursive(*second, target_id) {
                            Err(e) if e.to_string() == "DELETED" => Ok(new_first),
                            Ok(new_second) => {
                                // neither was the target, so we rebuild the split
                                Ok(LayoutNode::Split {
                                    direction,
                                    ratio,
                                    first: Box::new(new_first),
                                    second: Box::new(new_second),
                                    rect,
                                })
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }
    pub fn collect_leaf_layouts(&self) -> Vec<(usize, Rect)> {
        let mut layouts = Vec::new();
        Self::collect_layouts(&self.root, &mut layouts);
        layouts
    }

    fn collect_layouts(node: &LayoutNode, layouts: &mut Vec<(usize, Rect)>) {
        match node {
            LayoutNode::Leaf { pane_id, rect } => {
                layouts.push((*pane_id, *rect));
            }

            LayoutNode::Split { first, second, .. } => {
                Self::collect_layouts(first, layouts);
                Self::collect_layouts(second, layouts);
            }
        }
    }
}
