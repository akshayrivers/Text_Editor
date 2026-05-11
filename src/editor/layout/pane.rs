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
use crate::editor::uicomponents::View;
//NOTE:for now the pane is owning the Rect, needs to move into layout tree because it will be calculated there
pub enum PaneContent {
    TextView(View),
    // PluginView(PluginView),
    // FileExplorer(FileExplorerView),
    // Popup(PopupView),
}
pub struct Pane {
    pub pane_id: usize,
    pub content: PaneContent,
    pub active: bool,
}
