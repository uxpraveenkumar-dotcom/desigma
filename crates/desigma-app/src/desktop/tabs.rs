//! Open library documents in the current workspace and their in-window view state.

use crate::{app::{App, View}, theme};
use desigma_doc::NodeId;
use egui::{Rect, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo, WidgetType};

pub(crate) const HEIGHT: f32 = 38.0;

struct Tab {
    id: String,
    name: String,
    view: Option<TabView>,
}

struct TabView {
    page: NodeId,
    selection: Vec<NodeId>,
    viewport: View,
    inspect: bool,
}

#[derive(Default)]
pub(crate) struct Tabs {
    documents: Vec<Tab>,
    previews: crate::home::thumb::Textures,
}

impl App {
    pub(crate) fn remember_document_tab(&mut self) {
        if !cfg!(feature = "design-app") {
            return;
        }
        let Some(file) = &self.lib_file else { return };
        if self.desktop_tabs.documents.iter().all(|tab| tab.id != file.id) {
            self.desktop_tabs.documents.push(Tab { id: file.id.clone(), name: file.name.clone(), view: None });
        }
    }

    pub(crate) fn capture_tab_view(&mut self) {
        if self.home.is_some() {
            return;
        }
        let Some(file) = &self.lib_file else { return };
        if let Some(tab) = self.desktop_tabs.documents.iter_mut().find(|tab| tab.id == file.id) {
            tab.name.clone_from(&file.name);
            tab.view = Some(TabView {
                page: self.page,
                selection: self.selection.clone(),
                viewport: self.view,
                inspect: self.ui.dev_mode,
            });
        }
    }

    pub(crate) fn restore_tab_view(&mut self, id: &str) {
        self.ui.dev_mode = false;
        let Some(state) = self.desktop_tabs.documents.iter().find(|tab| tab.id == id).and_then(|tab| tab.view.as_ref()) else {
            return;
        };
        if self.doc.pages.contains(&state.page) {
            self.page = state.page;
            self.selection = state.selection.iter().copied().filter(|id| self.doc.page_of(*id) == Some(state.page)).collect();
            self.view = state.viewport;
            self.first_fit_done = true;
        }
        self.ui.dev_mode = state.inspect;
    }
}

enum Action {
    Home,
    Open(String),
    Close(String),
}

pub(crate) fn show(app: &mut App, ui: &mut Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0.0, theme::BG_SECONDARY);
    let mut bar = ui.new_child(UiBuilder::new().max_rect(rect.shrink2(Vec2::new(7.0, 5.0))));
    bar.set_clip_rect(rect);
    let mut action = None;
    let home = app.home.is_some();
    let active = app.lib_file.as_ref().filter(|_| !home);
    bar.horizontal(|ui| {
        if home_button(ui, home) && !home {
            action = Some(Action::Home);
        }
        ui.separator();
        egui::ScrollArea::horizontal().id_salt("desktop-document-tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                let Tabs { documents, previews } = &mut app.desktop_tabs;
                for tab in documents {
                    let current = active.filter(|file| file.id == tab.id);
                    let name = current.map_or(tab.name.as_str(), |file| file.name.as_str());
                    let (selected, closed) = document_button(ui, tab, name, current.is_some(), app.lib.as_ref(), previews);
                    if closed {
                        action = Some(Action::Close(tab.id.clone()));
                    } else if selected && current.is_none() {
                        action = Some(Action::Open(tab.id.clone()));
                    }
                }
            });
        });
    });
    match action {
        Some(Action::Home) => app.go_home(),
        Some(Action::Open(id)) => {
            app.open_library_file(&id);
            if app.home.is_none() && app.lib_file.as_ref().is_some_and(|file| file.id == id) {
                ui.memory_mut(|memory| {
                    if let Some(focused) = memory.focused() {
                        memory.surrender_focus(focused);
                    }
                });
            }
        }
        Some(Action::Close(id)) => close(app, id),
        None => {}
    }
}

fn tab_background(ui: &Ui, rect: Rect, active: bool, hovered: bool) {
    let color = if active { theme::BG_MENU } else if hovered { theme::BG_TERTIARY } else { theme::BG_SECONDARY };
    let painter = ui.painter();
    painter.rect_filled(rect, theme::cr(theme::RADIUS_MEDIUM), color);
    if active {
        painter.line_segment(
            [rect.left_bottom() + Vec2::new(5.0, -1.0), rect.right_bottom() + Vec2::new(-5.0, -1.0)],
            Stroke::new(2.0, theme::BORDER_SELECTED),
        );
    }
}

fn document_button(
    ui: &mut Ui, tab: &Tab, name: &str, active: bool,
    library: Option<&desigma_library::Library>, previews: &mut crate::home::thumb::Textures,
) -> (bool, bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(128.0, 26.0), Sense::hover());
    let mut label_rect = rect;
    if active {
        label_rect.max.x -= 24.0;
    }
    let response = ui.interact(label_rect, ui.id().with(("document-tab", &tab.id)), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, active, name));
    let hovered = response.hovered();
    tab_background(ui, rect, active, hovered);

    let color = if active || hovered { theme::TEXT } else { theme::TEXT_SECONDARY };
    let text_rect = label_rect.shrink2(Vec2::new(8.0, 0.0));
    ui.painter().with_clip_rect(text_rect).text(
        text_rect.left_center(), egui::Align2::LEFT_CENTER, name, theme::font_body(), color,
    );
    let open = response.clicked();
    let mut closed = false;
    if active {
        let close_rect = Rect::from_min_max(label_rect.right_top(), rect.right_bottom());
        let close = ui.interact(close_rect, ui.id().with(("close-document-tab", &tab.id)), Sense::click());
        close.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, format!("Close {name}")));
        if close.hovered() {
            ui.painter().rect_filled(close_rect.shrink(2.0), theme::cr(theme::RADIUS_MEDIUM), theme::BG_TERTIARY);
        }
        ui.painter().text(close_rect.center(), egui::Align2::CENTER_CENTER, "×", theme::font_body(), theme::TEXT);
        closed = close.on_hover_text(format!("Close {name}")).clicked();
    } else if !open {
        response.on_hover_ui(|ui| {
            use crate::home::time::{edited, wall_ms};
            ui.set_width(240.0);
            let entry = library.and_then(|lib| lib.file(&tab.id).ok());
            let thumbnail = library.zip(entry.as_ref()).and_then(|(lib, entry)| {
                let mut budget = 1;
                previews.get(ui.ctx(), lib, entry, &mut budget)
            });
            let (area, _) = ui.allocate_exact_size(Vec2::new(240.0, 135.0), Sense::hover());
            let painter = ui.painter();
            painter.rect_filled(area, 4.0, theme::BG_SECONDARY);
            if let Some((texture, size)) = thumbnail.filter(|(_, size)| size.x > 0.0 && size.y > 0.0) {
                let scale = (area.width() / size.x).min(area.height() / size.y);
                let image = Rect::from_center_size(area.center(), size * scale);
                painter.image(texture, image, Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            } else {
                painter.text(area.center(), egui::Align2::CENTER_CENTER, "No preview available", theme::font_body(), theme::TEXT_SECONDARY);
            }
            ui.label(name);
            if let Some(entry) = entry {
                ui.label(edited(wall_ms(), entry.modified));
            }
        });
    }
    (open, closed)
}

fn home_button(ui: &mut Ui, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(34.0, 26.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, active, "Home"));
    let painter = ui.painter();
    tab_background(ui, rect, active, response.hovered());
    let center = rect.center();
    let stroke = Stroke::new(1.4, if active || response.hovered() { theme::TEXT } else { theme::ICON_SECONDARY });
    let outline = [(-7.0, -1.0), (0.0, -7.0), (7.0, -1.0), (7.0, 7.0), (2.5, 7.0),
        (2.5, 2.0), (-2.5, 2.0), (-2.5, 7.0), (-7.0, 7.0), (-7.0, -1.0)];
    painter.add(egui::Shape::line(outline.into_iter().map(|(x, y)| center + Vec2::new(x, y)).collect(), stroke));
    response.on_hover_text("Home").clicked()
}

fn close(app: &mut App, id: String) {
    let active = app.home.is_none() && app.lib_file.as_ref().is_some_and(|file| file.id == id);
    if active {
        let again = id.clone();
        if !app.leave_ok(move |app| close(app, again)) {
            return;
        }
        app.go_home();
        if app.home.is_none() {
            return;
        }
    }
    app.desktop_tabs.documents.retain(|tab| tab.id != id);
}
