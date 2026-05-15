/*
So what should a pane contain?
If I think from the perspective of PHASE I the view for now is just one bigass Pane and it supports and need
    1. Buffer
    2. viewport and scrolling
    3. cursor tracking

Ultimately in this pane we will also track the 4. paneID 5. Active status and buffer will be just a reference or will have only buffer ID
hmmmm so a pane should just not support the buffer view and its functionality that is already being handled by the View, so pane should only
be concerned with what is inside and what size and geometry.
Pane should know:
    - what it displays
    - whether it is focused
    - geometry assigned to it
*/
use crate::{
    editor::uicomponents::{UIComponent, View},
    prelude::*,
};

pub enum PaneContent {
    TextView(View),
    PluginView(View),
    FileExplorer(View),
    Popup(View),
}

pub struct Pane {
    pub pane_id: usize,
    pub content: PaneContent,
    pub active: bool,
}

impl Pane {
    pub fn view(&self) -> Option<&View> {
        match &self.content {
            PaneContent::TextView(view)
            | PaneContent::PluginView(view)
            | PaneContent::FileExplorer(view)
            | PaneContent::Popup(view) => Some(view),
        }
    }

    pub fn view_mut(&mut self) -> Option<&mut View> {
        match &mut self.content {
            PaneContent::TextView(view)
            | PaneContent::PluginView(view)
            | PaneContent::FileExplorer(view)
            | PaneContent::Popup(view) => Some(view),
        }
    }
    pub fn resize(&mut self, rect: Rect) {
        match &mut self.content {
            PaneContent::TextView(view) => view.resize(rect),
            PaneContent::PluginView(view) => view.resize(rect),
            PaneContent::FileExplorer(view) => view.resize(rect),
            PaneContent::Popup(view) => view.resize(rect),
        }
    }
    pub fn render(&mut self) {
        let active = self.active;
        match &mut self.content {
            PaneContent::TextView(view)
            | PaneContent::PluginView(view)
            | PaneContent::FileExplorer(view)
            | PaneContent::Popup(view) => {
                view.set_active(active);
                view.render();
            }
        }
    }
}
