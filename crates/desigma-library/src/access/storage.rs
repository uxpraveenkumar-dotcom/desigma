//! Versioned snapshots written by the process that owns the shared catalog.

use std::{collections::HashMap, fs, io::ErrorKind, path::Path};

use serde::{Deserialize, Serialize};

use super::model::{Access, Organization, OrganizationAccess, Principal, ResourceKind};
use crate::{LibraryError, Result, write_atomic};

const VERSION: u32 = 1;

#[derive(Deserialize)]
struct Snapshot {
    version: u32,
    organizations: HashMap<String, Organization>,
}

#[derive(Serialize)]
struct SnapshotRef<'a> {
    version: u32,
    organizations: &'a HashMap<String, Organization>,
}

impl OrganizationAccess {
    /// Load a trusted snapshot. Platform admins come from runtime configuration.
    /// A missing file initializes an empty catalog; unreadable data returns an error.
    pub fn load(
        path: &Path,
        application_admins: impl IntoIterator<Item = String>,
    ) -> Result<Self> {
        let mut policy = Self::new(application_admins);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(policy),
            Err(error) => return Err(error.into()),
        };
        let snapshot: Snapshot = serde_json::from_slice(&bytes)?;
        if snapshot.version != VERSION {
            return Err(LibraryError::Invalid(format!(
                "unsupported access snapshot version: {}", snapshot.version
            )));
        }
        for org in snapshot.organizations.values() {
            validate(org)?;
        }
        policy.organizations = snapshot.organizations;
        Ok(policy)
    }

    /// Replace the snapshot atomically. The catalog owner must serialize writes
    /// to this path and keep it outside user-writable document storage.
    pub fn save(&self, path: &Path) -> Result<()> {
        let snapshot = SnapshotRef { version: VERSION, organizations: &self.organizations };
        write_atomic(path, &serde_json::to_vec_pretty(&snapshot)?)
    }
}

fn validate(org: &Organization) -> Result<()> {
    if org.admins.is_empty() || org.teams.values().any(|team| !team.members.contains(&team.lead)) {
        return Err(LibraryError::Invalid("invalid access snapshot membership".into()));
    }
    for resource in org.resources.values() {
        let valid_grants = resource.grants.keys().all(|principal| match principal {
            Principal::Account(_) => true,
            Principal::Team(team) => org.teams.contains_key(team),
        });
        if !valid_grants || (resource.limited && resource.kind != ResourceKind::Folder) {
            return Err(LibraryError::Invalid("invalid access snapshot grants".into()));
        }
        let mut ancestor = resource;
        for _ in 0..org.resources.len() {
            match (ancestor.kind, &ancestor.parent) {
                (ResourceKind::Project, None) => break,
                (ResourceKind::Folder | ResourceKind::File, Some(parent)) => {
                    ancestor = org.resources.get(parent)
                        .filter(|parent| parent.kind != ResourceKind::File)
                        .ok_or_else(invalid_hierarchy)?;
                }
                _ => return Err(invalid_hierarchy()),
            }
        }
        if ancestor.kind != ResourceKind::Project || ancestor.parent.is_some() {
            return Err(invalid_hierarchy());
        }
    }
    Ok(())
}

fn invalid_hierarchy() -> LibraryError {
    LibraryError::Invalid("invalid access snapshot hierarchy".into())
}

pub(super) mod grants {
    use super::{Access, HashMap, Principal};
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};

    pub fn serialize<S: Serializer>(value: &HashMap<Principal, Access>, serializer: S) -> Result<S::Ok, S::Error> {
        value.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<HashMap<Principal, Access>, D::Error> {
        let entries = Vec::<(Principal, Access)>::deserialize(deserializer)?;
        let mut grants = HashMap::with_capacity(entries.len());
        for (principal, access) in entries {
            if grants.insert(principal, access).is_some() {
                return Err(D::Error::custom("duplicate access grant"));
            }
        }
        Ok(grants)
    }
}
