//! Clipboard formats shared with the browser, Finder, and other applications.

use std::{io::Cursor, path::PathBuf};

use crate::{app::App, canvas::{create, interact}, edit::Tx, layout};
use desigma_doc::{Command, NodeKind};

pub fn paste(app: &mut App, text_event: Option<&str>) {
    paste_at(app, text_event, None);
}

pub fn paste_at(app: &mut App, text_event: Option<&str>, at: Option<egui::Pos2>) {
    let point = at.map(|pos| app.view.to_page(app.canvas_rect.min, pos));
    let center = point.unwrap_or_else(|| app.view.to_page(app.canvas_rect.min, app.canvas_rect.center()));
    if app.read_only() {
        return;
    }
    if text_event.is_some_and(is_design_text) {
        app.paste_at(text_event, point);
        return;
    }
    // Synthetic paste events in tests must not depend on the workstation clipboard.
    let mut clipboard = if cfg!(test) && text_event.is_some() {
        None
    } else {
        arboard::Clipboard::new().ok()
    };
    #[cfg(target_os = "macos")]
    if !cfg!(test) || text_event.is_none() {
        if let Some(svg) = super::macos::svg() {
            app.paste_at(Some(&svg), point);
            return;
        }
    }
    let clipboard_text = clipboard.as_mut().and_then(|clipboard| clipboard.get_text().ok());
    if let Some(text) = clipboard_text.as_deref().filter(|text| is_design_text(text)) {
        app.paste_at(Some(text), point);
        return;
    }
    if let Some(clipboard) = &mut clipboard {
        if let Ok(paths) = clipboard.get().file_list() {
            if !paths.is_empty() {
                paste_files(app, paths, at);
                return;
            }
        }
        if let Ok(image) = clipboard.get_image() {
            let bytes = u32::try_from(image.width).ok().zip(u32::try_from(image.height).ok())
                .and_then(|(width, height)| image::RgbaImage::from_raw(width, height, image.bytes.into_owned()));
            if let Some(bytes) = bytes {
                let mut png = Cursor::new(Vec::new());
                match image::DynamicImage::ImageRgba8(bytes).write_to(&mut png, image::ImageFormat::Png) {
                    Ok(()) => {
                        interact::place_image_bytes(app, center, "Pasted image", png.into_inner());
                    }
                    Err(error) => app.show_toast(format!("Cannot paste image: {error}"), true),
                }
            } else {
                app.show_toast("Cannot paste image: invalid dimensions", true);
            }
            return;
        }
    }

    let system_text = text_event.map(str::to_owned).or(clipboard_text);
    match system_text.as_deref().filter(|text| !text.trim().is_empty()) {
        Some(text) if is_design_text(text) => app.paste_at(Some(text), point),
        Some(text) => paste_text(app, text, center),
        None if clipboard.is_none() => app.paste_at(None, point),
        None => app.show_toast("No supported clipboard content", false),
    }
}

fn is_design_text(text: &str) -> bool {
    desigma_svg::looks_like_svg(text) || super::from_json(text).is_some()
}

fn paste_text(app: &mut App, text: &str, center: desigma_doc::Vec2) {
    let parent = app.container_for_insert();
    let Some((mut node, _)) = create::new_text(app, parent, center) else { return };
    if let NodeKind::Text(props) = &mut node.kind {
        props.content = text.replace("\r\n", "\n").replace('\r', "\n");
        node.size = layout::estimate_text_size(props);
    }
    let id = node.id;
    let index = app.doc.get(parent).map_or(0, |node| node.children.len());
    let mut tx = Tx::new();
    if let Err(error) = tx.cmd(&mut app.doc, Command::Insert { parent, index, nodes: vec![node] }) {
        app.show_toast(format!("Cannot paste text: {error}"), true);
        return;
    }
    layout::relayout_around(&mut app.doc, &mut tx.ed, parent);
    if app.run_tx(tx) {
        app.select_only(id);
        app.fonts.load_for(&app.doc);
    }
}

fn paste_files(app: &mut App, paths: Vec<PathBuf>, at: Option<egui::Pos2>) {
    let mut designs = Vec::new();
    let mut unsupported = 0;
    let center = app.view.to_page(app.canvas_rect.min, at.unwrap_or_else(|| app.canvas_rect.center()));
    for path in paths {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default().to_ascii_lowercase();
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("Pasted file");
        match extension.as_str() {
            "png" | "jpg" | "jpeg" | "webp" => match std::fs::read(&path) {
                Ok(bytes) => { interact::place_image_bytes(app, center, name, bytes); }
                Err(error) => app.show_toast(format!("Cannot read {name}: {error}"), true),
            },
            "svg" => match std::fs::read_to_string(&path) {
                Ok(svg) => super::drop_svg(app, &svg, name, at, 0),
                Err(error) => app.show_toast(format!("Cannot read {name}: {error}"), true),
            },
            "fig" | "desigma" | "pen" | "json" => designs.push(path),
            _ => unsupported += 1,
        }
    }
    if let Some(first) = designs.first() {
        if designs.len() == 1 {
            app.request_open(crate::instance::Open::File(first.clone()));
        } else {
            let target = app.lib_file.as_ref().and_then(|file| app.lib.as_ref()?.file(&file.id).ok());
            if let Some(entry) = target {
                app.import_into_library(&designs, &entry.project, entry.folder.as_deref());
            } else {
                app.show_toast("Open a project design file before pasting multiple documents", true);
            }
        }
    }
    if unsupported > 0 {
        app.show_toast(format!("{unsupported} clipboard file(s) use an unsupported format"), true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use desigma_doc::Document;

    #[test]
    fn external_plain_text_creates_an_editable_layer_and_undoes() {
        let mut doc = Document::new("Destination");
        let page = doc.add_page("Page 1");
        let mut app = App::new(doc);
        app.canvas_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        paste_text(&mut app, "Copied from the web\r\nSecond line", desigma_doc::Vec2::new(40.0, 60.0));
        let layer = app.doc.get(app.selection[0]).unwrap();
        assert_eq!(layer.kind.any_text().unwrap().content, "Copied from the web\nSecond line");
        app.undo();
        assert!(app.doc.get(page).unwrap().children.is_empty());
    }
}
