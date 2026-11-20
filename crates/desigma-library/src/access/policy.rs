use std::collections::{HashMap, HashSet};

use super::model::{Access, AccessError, Organization, OrganizationAccess, Principal, Resource, ResourceKind, Team};

impl OrganizationAccess {
    pub fn new(application_admins: impl IntoIterator<Item = String>) -> Self {
        Self {
            application_admins: application_admins.into_iter().collect(),
            organizations: HashMap::new(),
        }
    }

    pub fn create_organization(
        &mut self,
        actor: &str,
        organization: String,
        first_admin: String,
    ) -> Result<(), AccessError> {
        if !self.application_admins.contains(actor) {
            return Err(AccessError::Forbidden);
        }
        insert_new(
            &mut self.organizations,
            organization,
            Organization { admins: HashSet::from([first_admin]), ..Organization::default() },
        )
    }

    pub fn create_team(
        &mut self,
        actor: &str,
        organization: &str,
        team: String,
        lead: String,
    ) -> Result<(), AccessError> {
        let org = self.administer(actor, organization)?;
        insert_new(&mut org.teams, team, Team { members: HashSet::from([lead.clone()]), lead })
    }

    pub fn set_team_member(
        &mut self,
        actor: &str,
        organization: &str,
        team: &str,
        account: String,
        member: bool,
    ) -> Result<(), AccessError> {
        let org = self.organizations.get_mut(organization).ok_or(AccessError::NotFound)?;
        let team = org.teams.get_mut(team).ok_or(AccessError::NotFound)?;
        if !org.admins.contains(actor) && team.lead != actor {
            return Err(AccessError::Forbidden);
        }
        if member {
            team.members.insert(account);
        } else if team.lead != account {
            team.members.remove(&account);
        } else {
            return Err(AccessError::Forbidden);
        }
        Ok(())
    }

    pub fn create_resource(
        &mut self,
        actor: &str,
        organization: &str,
        id: String,
        kind: ResourceKind,
        parent: Option<&str>,
    ) -> Result<(), AccessError> {
        let org = self.administer(actor, organization)?;
        match (kind, parent) {
            (ResourceKind::Project, None) => {}
            (ResourceKind::Folder | ResourceKind::File, Some(parent))
                if org.resources.get(parent).is_some_and(|r| r.kind != ResourceKind::File) => {}
            _ => return Err(AccessError::InvalidHierarchy),
        }
        insert_new(
            &mut org.resources,
            id,
            Resource {
                kind,
                parent: parent.map(str::to_owned),
                limited: false,
                grants: HashMap::new(),
            },
        )
    }

    pub fn set_limited(
        &mut self,
        actor: &str,
        organization: &str,
        folder: &str,
        limited: bool,
    ) -> Result<(), AccessError> {
        let org = self.administer(actor, organization)?;
        let resource = org.resources.get_mut(folder).ok_or(AccessError::NotFound)?;
        if resource.kind != ResourceKind::Folder {
            return Err(AccessError::InvalidHierarchy);
        }
        resource.limited = limited;
        Ok(())
    }

    /// A missing access value revokes the direct grant.
    pub fn set_grant(
        &mut self,
        actor: &str,
        organization: &str,
        resource: &str,
        principal: Principal,
        access: Option<Access>,
    ) -> Result<(), AccessError> {
        let org = self.administer(actor, organization)?;
        if let Principal::Team(team) = &principal {
            if !org.teams.contains_key(team) {
                return Err(AccessError::NotFound);
            }
        }
        let resource = org.resources.get_mut(resource).ok_or(AccessError::NotFound)?;
        match access {
            Some(access) => { resource.grants.insert(principal, access); }
            None => { resource.grants.remove(&principal); }
        }
        Ok(())
    }

    /// Resolve permissions from the closest access boundary down to the target.
    /// Administrative roles do not themselves grant document access.
    pub fn effective_access(&self, actor: &str, organization: &str, resource: &str) -> Option<Access> {
        let org = self.organizations.get(organization)?;
        let mut resource = org.resources.get(resource)?;
        let mut access = None;
        loop {
            let direct = resource.grants.get(&Principal::Account(actor.to_owned())).copied();
            let team = resource.grants.iter().filter_map(|(principal, access)| {
                let Principal::Team(team) = principal else { return None };
                org.teams.get(team)?.members.contains(actor).then_some(*access)
            }).max();
            access = access.max(direct).max(team);
            if resource.limited || access == Some(Access::Edit) {
                return access;
            }
            let Some(parent) = &resource.parent else { return access };
            resource = org.resources.get(parent)?;
        }
    }

    fn administer(&mut self, actor: &str, organization: &str) -> Result<&mut Organization, AccessError> {
        let org = self.organizations.get_mut(organization).ok_or(AccessError::NotFound)?;
        if org.admins.contains(actor) {
            Ok(org)
        } else {
            Err(AccessError::Forbidden)
        }
    }
}

fn insert_new<T>(map: &mut HashMap<String, T>, id: String, value: T) -> Result<(), AccessError> {
    use std::collections::hash_map::Entry;
    match map.entry(id) {
        Entry::Occupied(_) => Err(AccessError::AlreadyExists),
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
    }
}
