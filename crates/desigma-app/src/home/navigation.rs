//! Browser history and location controls.

use super::{Act, Home, Section};
use crate::{icons::{self, Icon}, theme, widgets};
use egui::{Pos2, Rect, Ui, Vec2};

impl Home {
    pub(super) fn navigate(&mut self, section: Section) {
        if self.section != section {
            self.back.push(self.section.clone());
            self.forward.clear();
        }
        self.set_location(section);
    }

    pub(super) fn step(&mut self, back: bool) {
        let (from, to) = if back { (&mut self.back, &mut self.forward) } else { (&mut self.forward, &mut self.back) };
        if let Some(location) = from.pop() {
            to.push(self.section.clone());
            self.set_location(location);
        }
    }

    fn set_location(&mut self, section: Section) {
        self.section = section;
        self.search.clear();
        self.selected = None;
        self.scroll = 0.0;
        self.menu = None;
        self.sort_menu = None;
    }
}

pub(super) fn controls(home: &Home, ui: &mut Ui, start: Pos2, actions: &mut Vec<Act>) {
    for (index, (icon, enabled, label, act)) in [
        (Icon::ChevronLeft, !home.back.is_empty(), "Back", Act::Back),
        (Icon::ChevronRight, !home.forward.is_empty(), "Forward", Act::Forward),
    ].into_iter().enumerate() {
        let rect = Rect::from_min_size(start + Vec2::new(index as f32 * 30.0, 0.0), Vec2::splat(28.0));
        if enabled {
            if widgets::icon_button(ui, rect, ("home-history", index), icon, false, label).clicked() {
                actions.push(act);
            }
        } else {
            icons::paint(ui.painter(), icon, rect.center(), 14.0, theme::TEXT_DISABLED);
        }
        widgets::record(ui, if index == 0 { "home-back" } else { "home-forward" }, rect);
    }
}
