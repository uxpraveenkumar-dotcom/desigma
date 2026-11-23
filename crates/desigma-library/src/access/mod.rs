//! Authorization rules for shared organizations. The caller supplies an authenticated
//! account ID. Local library files and collaboration sessions do not use these rules yet.

mod model;
mod policy;
mod storage;

#[cfg(test)]
mod tests;

pub use model::{Access, AccessError, OrganizationAccess, Principal, ResourceKind};
