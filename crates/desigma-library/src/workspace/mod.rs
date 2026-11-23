//! Local organization catalog. Each organization owns a separate projects library.

use std::{fs, io::ErrorKind, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::{Library, LibraryError, Result, new_id, write_atomic};

#[derive(Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Catalog {
    organizations: Vec<Organization>,
    active: Option<String>,
}

pub struct Workspace {
    local_root: PathBuf,
    catalog: Catalog,
}

impl Workspace {
    pub fn open(local_root: PathBuf) -> Result<Self> {
        let catalog: Catalog = match fs::read(Self::catalog_path(&local_root)) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == ErrorKind::NotFound => Catalog::default(),
            Err(error) => return Err(error.into()),
        };
        let mut ids = std::collections::HashSet::new();
        if catalog.organizations.iter().any(|org| {
            org.id.is_empty() || !org.id.bytes().all(|c| c.is_ascii_hexdigit()) || !ids.insert(org.id.as_str())
        }) || catalog.active.as_deref().is_some_and(|id| !ids.contains(id)) {
            return Err(LibraryError::Invalid("Invalid organization catalog".into()));
        }
        Ok(Self { local_root, catalog })
    }

    pub fn organizations(&self) -> &[Organization] {
        &self.catalog.organizations
    }

    pub fn active(&self) -> Option<&str> {
        self.catalog.active.as_deref()
    }

    pub fn active_root(&self) -> PathBuf {
        self.root_for(self.active())
    }

    pub fn create(&mut self, name: &str) -> Result<Library> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LibraryError::Invalid("Enter an organization name".into()));
        }
        if self.organizations().iter().any(|org| org.name.to_lowercase() == name.to_lowercase()) {
            return Err(LibraryError::Invalid("An organization with that name already exists".into()));
        }
        let mut next = self.catalog.clone();
        let id = new_id();
        next.organizations.push(Organization { id: id.clone(), name: name.to_owned() });
        next.active = Some(id);
        self.apply(next)
    }

    pub fn select(&mut self, id: Option<String>) -> Result<Library> {
        if id.as_deref().is_some_and(|id| !self.organizations().iter().any(|org| org.id == id)) {
            return Err(LibraryError::NotFound("organization".into()));
        }
        let mut next = self.catalog.clone();
        next.active = id;
        self.apply(next)
    }

    fn apply(&mut self, next: Catalog) -> Result<Library> {
        let library = Library::open(self.root_for(next.active.as_deref()))?;
        let path = Self::catalog_path(&self.local_root);
        // Commit the selection before updating the active workspace in memory.
        write_atomic(&path, &serde_json::to_vec_pretty(&next)?)?;
        self.catalog = next;
        Ok(library)
    }

    fn catalog_path(root: &Path) -> PathBuf {
        root.join("organizations.json")
    }

    fn root_for(&self, id: Option<&str>) -> PathBuf {
        match id {
            Some(id) => self.local_root.join("organizations").join(id),
            None => self.local_root.clone(),
        }
    }
}

#[cfg(test)]
mod tests;
