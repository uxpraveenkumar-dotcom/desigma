//! The organization’s project cards and list rows use native home styling.

use std::collections::HashMap;

use crate::{
    home::{Act, Home, Section, Sort, Target, ViewMode, time::{relative, wall_ms}},
    icons::{self, Icon},
    theme,
    widgets,
};
use desigma_library::{FileEntry, Library, Project};
use egui::{Align2, Rect, Sense, Ui, UiBuilder, Vec2};

pub(crate) fn show(lib: &Library, home: &mut Home, ui: &mut Ui, rect: Rect, actions: &mut Vec<Act>) {
    let mut content = ui.new_child(UiBuilder::new().max_rect(rect.shrink(20.0)));
    content.set_clip_rect(rect);
    if home.snap.projects.is_empty() {
        content.label("No projects yet");
        if content.button("Create your first project").clicked() {
            actions.push(Act::NewProject);
        }
        return;
    }

    let mut files: HashMap<&str, Vec<&FileEntry>> = HashMap::new();
    for file in &home.snap.files {
        files.entry(file.project.as_str()).or_default().push(file);
    }
    for entries in files.values_mut() {
        entries.sort_by_key(|file| std::cmp::Reverse(file.modified));
    }
    let mut projects: Vec<_> = home.snap.projects.iter().collect();
    match home.sort {
        Sort::Name => projects.sort_by_key(|project| project.name.to_lowercase()),
        Sort::LastModified => projects.sort_by_key(|project| std::cmp::Reverse(modified(project, files.get(project.id.as_str())))),
        Sort::LastOpened => projects.sort_by_key(|project| {
            std::cmp::Reverse(files.get(project.id.as_str()).and_then(|entries| entries.iter().map(|file| file.opened).max()).unwrap_or(0))
        }),
    }
    let list = home.view == ViewMode::List;
    let width = content.available_width().max(1.0);
    let card_width = width.min(300.0);
    let columns = if list { 1 } else { ((width + 12.0) / (card_width + 12.0)).floor().max(1.0) as usize };
    let now = wall_ms();
    let mut budget = 8;
    egui::ScrollArea::vertical().id_salt("workspace-project-list").show(&mut content, |ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(12.0);
        for row in projects.chunks(columns) {
            ui.horizontal(|ui| {
                for project in row {
                    let entries = files.get(project.id.as_str());
                    let title = &project.name;
                    let (area, response) = ui.allocate_exact_size(
                        Vec2::new(if list { width } else { card_width }, if list { 48.0 } else { 136.0 }),
                        Sense::click(),
                    );
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title.as_str()));
                    if response.clicked() {
                        actions.push(Act::Go(Section::Project(project.id.clone(), None)));
                    }
                    if response.secondary_clicked() {
                        home.menu = Some((Target::Project(project.id.clone()), response.interact_pointer_pos().unwrap_or(area.left_bottom())));
                    }
                    if !ui.is_rect_visible(area) {
                        continue;
                    }

                    let p = ui.painter().with_clip_rect(area);
                    p.rect_filled(area, theme::cr(theme::RADIUS_MEDIUM), if response.hovered() { theme::BG_TERTIARY } else { theme::BG_SECONDARY });
                    if !list {
                        let preview_width = (area.width() - 32.0) / 3.0;
                        for index in 0..3 {
                            let pos = area.min + Vec2::new(10.0 + index as f32 * (preview_width + 6.0), 10.0);
                            let preview = Rect::from_min_size(pos, Vec2::new(preview_width, 70.0));
                            p.rect_filled(preview, 5.0, theme::BG);
                            if let Some((texture, size)) = entries.and_then(|entries| entries.get(index))
                                .and_then(|entry| home.thumbs.get(ui.ctx(), lib, entry, &mut budget))
                                .filter(|(_, size)| size.x > 0.0 && size.y > 0.0)
                            {
                                let scale = (preview.width() / size.x).min(preview.height() / size.y);
                                p.image(texture, Rect::from_center_size(preview.center(), size * scale),
                                    Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
                            }
                        }
                    }
                    let base = area.min + Vec2::new(34.0, if list { 5.0 } else { 88.0 });
                    icons::paint(&p, Icon::Folder, base + Vec2::new(-13.0, 13.0), 16.0, theme::ICON_SECONDARY);
                    p.text(base, Align2::LEFT_TOP, title, theme::font_body_strong(), theme::TEXT);
                    p.text(base + Vec2::new(0.0, 19.0), Align2::LEFT_TOP,
                        format!("Updated {}", relative(now, modified(project, entries))), theme::font_body(), theme::TEXT_SECONDARY);
                    widgets::record(ui, &format!("project-card-{}", project.id), area);
                }
            });
        }
    });
}

fn modified(project: &Project, files: Option<&Vec<&FileEntry>>) -> u64 {
    files.and_then(|entries| entries.first()).map_or(project.modified, |file| project.modified.max(file.modified))
}
