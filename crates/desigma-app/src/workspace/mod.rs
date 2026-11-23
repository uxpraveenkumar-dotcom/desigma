//! Organization selection for the native home screen.

pub(crate) mod ui;
pub(crate) mod projects;

use crate::{app::App, home::{Home, Section}};
use desigma_library::{Library, workspace::Workspace};
use std::path::PathBuf;

impl App {
    pub(crate) fn workspace_root(&mut self, local_root: PathBuf) -> PathBuf {
        match Workspace::open(local_root.clone()) {
            Ok(workspace) => {
                let root = workspace.active_root();
                self.workspace = Some(workspace);
                root
            }
            Err(error) => {
                self.show_toast(format!("Cannot open organizations: {error}"), true);
                local_root
            }
        }
    }

    pub(crate) fn switch_organization(&mut self, id: Option<String>) {
        if let Some(workspace) = self.workspace.as_mut() {
            let result = workspace.select(id);
            self.apply_workspace(result);
        }
    }

    pub(crate) fn create_organization(&mut self, name: &str) -> bool {
        let Some(workspace) = self.workspace.as_mut() else { return false };
        let result = workspace.create(name);
        self.apply_workspace(result)
    }

    fn apply_workspace(&mut self, result: desigma_library::Result<Library>) -> bool {
        let library = match result {
            Ok(library) => library,
            Err(error) => {
                self.show_toast(error.to_string(), true);
                return false;
            }
        };
        self.lib_file = None;
        self.desktop_tabs = Default::default();
        self.parked_home = None;
        self.autosave = Default::default();
        self.libs.set_root(library.root().to_path_buf());
        let mut home = Home::new();
        home.section = Section::Projects;
        home.refresh(&library);
        self.lib = Some(library);
        self.home = Some(home);
        true
    }
}
