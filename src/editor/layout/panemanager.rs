use crate::{
    editor::layout::{Pane, PaneContent},
    prelude::*,
};
use std::collections::HashMap;

#[derive(Default)]
pub struct PaneManager {
    panes: HashMap<usize, Pane>,
    active_pane: usize,
    next_pane_id: usize,
}

impl PaneManager {
    pub fn new(initial_pane: Pane) -> Self {
        let initial_pane_id = initial_pane.pane_id;

        let mut panes = HashMap::new();
        panes.insert(initial_pane_id, initial_pane);

        Self {
            panes,
            active_pane: initial_pane_id,
            next_pane_id: initial_pane_id + 1,
        }
    }

    pub fn create_pane(&mut self, content: PaneContent, rect: Rect) -> usize {
        let pane_id = self.next_pane_id;

        self.next_pane_id += 1;

        let pane = Pane {
            pane_id,
            content,
            active: false,
        };

        self.panes.insert(pane_id, pane);

        pane_id
    }

    pub fn get_pane(&self, pane_id: usize) -> Option<&Pane> {
        self.panes.get(&pane_id)
    }

    pub fn get_pane_mut(&mut self, pane_id: usize) -> Option<&mut Pane> {
        self.panes.get_mut(&pane_id)
    }

    pub fn active_pane(&self) -> Option<&Pane> {
        self.panes.get(&self.active_pane)
    }

    pub fn active_pane_mut(&mut self) -> Option<&mut Pane> {
        self.panes.get_mut(&self.active_pane)
    }

    pub fn set_active_pane(&mut self, pane_id: usize) {
        if let Some(current_active) = self.panes.get_mut(&self.active_pane) {
            current_active.active = false;
        }

        if let Some(new_active) = self.panes.get_mut(&pane_id) {
            new_active.active = true;
        }

        self.active_pane = pane_id;
    }

    pub fn remove_pane(&mut self, pane_id: usize) -> Option<Pane> {
        self.panes.remove(&pane_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pane> {
        self.panes.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Pane> {
        self.panes.values_mut()
    }
}
