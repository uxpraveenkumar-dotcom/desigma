//! Inline editing for the frame name above the canvas.

use crate::{app::App, theme, widgets};
use desigma_doc::{NodeId, NodeKind};
use egui::{Id, Key, Rect, Stroke, StrokeKind, TextEdit, Ui, Vec2, text::{CCursor, CCursorRange}};

pub(super) struct FrameRename {
    id: NodeId,
    text: String,
    fresh: bool,
}

pub(super) fn begin(app: &mut App, id: NodeId) -> bool {
    let Some(node) = app.doc.get(id).filter(|node| matches!(node.kind, NodeKind::Frame(_)) && !node.locked) else {
        return false;
    };
    let text = node.name.clone();
    app.select_only(id);
    app.canvas.frame_rename = Some(FrameRename { id, text, fresh: true });
    true
}

pub(super) fn show(app: &mut App, ui: &mut Ui, canvas: Rect) {
    let Some(mut edit) = app.canvas.frame_rename.take() else { return };
    let Some((_, label)) = super::draw::label_rects(app, canvas).into_iter().find(|(id, _)| *id == edit.id) else { return };
    let id = Id::new(("canvas-frame-rename", edit.id.0));
    if app.read_only() {
        ui.memory_mut(|memory| memory.surrender_focus(id));
        return;
    }
    let width = (label.width() + 20.0).max(132.0).min(canvas.width());
    let mut area = Rect::from_center_size(label.left_center() + Vec2::new(width / 2.0, 0.0), Vec2::new(width, 25.0));
    let x = area.left().clamp(canvas.left(), (canvas.right() - width).max(canvas.left()));
    area = area.translate(Vec2::new(x - area.left(), 0.0));
    let painter = ui.painter().with_clip_rect(canvas);
    let rounding = theme::cr(theme::RADIUS_MEDIUM);
    painter.rect_filled(area, rounding, theme::BG);
    painter.rect_stroke(area, rounding, Stroke::new(1.0, theme::BORDER_SELECTED), StrokeKind::Inside);
    let input = area.shrink2(Vec2::new(5.0, 2.0));
    let response = ui.put(input, TextEdit::singleline(&mut edit.text).id(id).frame(egui::Frame::NONE)
        .font(theme::font_body()).text_color(theme::TEXT).min_size(input.size()).desired_width(input.width()));
    widgets::record(ui, "canvas-frame-rename", area);
    if edit.fresh {
        response.request_focus();
        let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(edit.text.chars().count()))));
        state.store(ui.ctx(), id);
        edit.fresh = false;
        app.canvas.frame_rename = Some(edit);
        return;
    }
    let (cancel, commit) = ui.input(|input| (
        input.key_pressed(Key::Escape),
        input.key_pressed(Key::Enter) || response.lost_focus()
            || (input.pointer.any_pressed() && input.pointer.interact_pos().is_some_and(|pos| !area.contains(pos))),
    ));
    if cancel || commit {
        ui.memory_mut(|memory| memory.surrender_focus(id));
        if !cancel && app.doc.get(edit.id).is_some_and(|node| node.name != edit.text.trim()) {
            app.rename(edit.id, &edit.text);
        }
    } else {
        app.canvas.frame_rename = Some(edit);
    }
}
