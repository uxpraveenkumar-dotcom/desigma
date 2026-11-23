//! The organization selector reuses the native sidebar and menu styling.

use crate::{home::Act, theme, widgets::menu::{MenuItem, MenuResult, popup_menu}};
use desigma_library::workspace::Workspace;
use egui::{Align2, Id, Rect, Sense, Ui, Vec2};

pub(crate) fn switcher(workspace: &Workspace, ui: &mut Ui, rect: Rect, anchor: &mut Option<Rect>, actions: &mut Vec<Act>) {
    let active = workspace.active();
    let organizations = workspace.organizations();
    let label = organizations.iter().find(|org| Some(org.id.as_str()) == active)
        .map_or("Local workspace", |org| org.name.as_str());
    let response = ui.interact(rect, Id::new("workspace-switcher"), Sense::click());
    let painter = ui.painter();
    crate::widgets::button_bg(painter, rect, &response, anchor.is_some(), theme::cr(theme::RADIUS_MEDIUM));
    painter.with_clip_rect(Rect::from_min_max(rect.min + Vec2::new(12.0, 0.0), rect.max - Vec2::new(34.0, 0.0))).text(
        rect.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER,
        label, theme::font_body_strong(), theme::TEXT,
    );
    crate::icons::paint(painter, crate::icons::Icon::ChevronDown, rect.right_center() - Vec2::new(16.0, 0.0), 12.0, theme::ICON_SECONDARY);
    if response.clicked() {
        *anchor = if anchor.is_some() { None } else { Some(rect) };
    }
    let Some(menu_rect) = *anchor else { return };
    let mut entries = vec![MenuItem::new("Local workspace").checked(active.is_none())];
    entries.extend(organizations.iter().map(|org| MenuItem::new(org.name.as_str()).checked(Some(org.id.as_str()) == active)));
    entries.push(MenuItem::new("New organization"));
    match popup_menu(ui.ctx(), Id::new("workspace-switcher-menu"), menu_rect.left_bottom(), rect.width(), &entries, Some(menu_rect)) {
        MenuResult::Picked(index) => {
            *anchor = None;
            if index == entries.len() - 1 {
                actions.push(Act::NewOrganization);
            } else {
                let id = index.checked_sub(1).map(|i| organizations[i].id.clone());
                if id.as_deref() != active {
                    actions.push(Act::SwitchOrganization(id));
                }
            }
        }
        MenuResult::Closed => *anchor = None,
        MenuResult::Open => {}
    }
}
