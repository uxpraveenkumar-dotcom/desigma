use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
pub enum Access {
    View,
    Edit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum ResourceKind {
    Project,
    Folder,
    File,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum Principal {
    Account(String),
    Team(String),
}

#[derive(Debug, Eq, PartialEq)]
pub enum AccessError {
    Forbidden,
    InvalidHierarchy,
    AlreadyExists,
    NotFound,
}

#[derive(Default, Deserialize, Serialize)]
pub(super) struct Team {
    pub(super) lead: String,
    pub(super) members: HashSet<String>,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Resource {
    pub(super) kind: ResourceKind,
    pub(super) parent: Option<String>,
    pub(super) limited: bool,
    #[serde(with = "super::storage::grants")]
    pub(super) grants: HashMap<Principal, Access>,
}

#[derive(Default, Deserialize, Serialize)]
pub(super) struct Organization {
    pub(super) admins: HashSet<String>,
    pub(super) teams: HashMap<String, Team>,
    pub(super) resources: HashMap<String, Resource>,
}

/// Shared catalog policy. IDs are scoped to the calling organization.
pub struct OrganizationAccess {
    pub(super) application_admins: HashSet<String>,
    pub(super) organizations: HashMap<String, Organization>,
}
