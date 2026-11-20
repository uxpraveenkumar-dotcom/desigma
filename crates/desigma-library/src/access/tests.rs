use super::*;

#[test]
fn limited_folders_stop_inheritance_and_team_removal_revokes_access() {
    let mut policy = OrganizationAccess::new(["platform".into()]);
    policy.create_organization("platform", "org".into(), "admin".into()).unwrap();
    policy.create_team("admin", "org", "design".into(), "lead".into()).unwrap();
    policy.set_team_member("lead", "org", "design", "editor".into(), true).unwrap();
    policy.create_resource("admin", "org", "project".into(), ResourceKind::Project, None).unwrap();
    policy.create_resource("admin", "org", "folder".into(), ResourceKind::Folder, Some("project")).unwrap();
    policy.create_resource("admin", "org", "file".into(), ResourceKind::File, Some("folder")).unwrap();

    let team = Principal::Team("design".into());
    policy.set_grant("admin", "org", "project", team.clone(), Some(Access::Edit)).unwrap();
    assert_eq!(policy.effective_access("editor", "org", "file"), Some(Access::Edit));

    policy.set_limited("admin", "org", "folder", true).unwrap();
    assert_eq!(policy.effective_access("editor", "org", "file"), None);
    assert_eq!(policy.effective_access("admin", "org", "file"), None);
    policy.set_grant("admin", "org", "folder", team, Some(Access::View)).unwrap();
    assert_eq!(policy.effective_access("editor", "org", "file"), Some(Access::View));
    policy.set_team_member("lead", "org", "design", "editor".into(), false).unwrap();
    assert_eq!(policy.effective_access("editor", "org", "file"), None);
}

#[test]
fn organizations_cannot_inherit_each_others_projects_or_grants() {
    let mut policy = OrganizationAccess::new(["platform".into()]);
    assert_eq!(
        policy.create_organization("member", "one".into(), "member".into()),
        Err(AccessError::Forbidden)
    );
    policy.create_organization("platform", "one".into(), "admin-one".into()).unwrap();
    policy.create_organization("platform", "two".into(), "admin-two".into()).unwrap();
    policy.create_resource("admin-one", "one", "project".into(), ResourceKind::Project, None).unwrap();
    assert_eq!(
        policy.create_resource("admin-two", "two", "file".into(), ResourceKind::File, Some("project")),
        Err(AccessError::InvalidHierarchy)
    );
    assert_eq!(
        policy.create_resource("admin-one", "two", "project".into(), ResourceKind::Project, None),
        Err(AccessError::Forbidden)
    );
    policy.create_resource("admin-two", "two", "project".into(), ResourceKind::Project, None).unwrap();
    policy.set_grant(
        "admin-one", "one", "project", Principal::Account("guest".into()), Some(Access::Edit),
    ).unwrap();
    assert_eq!(policy.effective_access("guest", "one", "project"), Some(Access::Edit));
    assert_eq!(policy.effective_access("guest", "two", "project"), None);
}

#[test]
fn access_snapshot_preserves_grants_and_uses_runtime_platform_admins() {
    let dir = std::env::temp_dir().join(format!("desigma-access-{}", crate::new_id()));
    let path = dir.join("catalog.json");
    let mut policy = OrganizationAccess::load(&path, ["old-admin".into()]).unwrap();
    policy.create_organization("old-admin", "org".into(), "org-admin".into()).unwrap();
    policy.create_team("org-admin", "org", "design".into(), "lead".into()).unwrap();
    policy.create_resource("org-admin", "org", "project".into(), ResourceKind::Project, None).unwrap();
    policy.set_grant(
        "org-admin", "org", "project", Principal::Team("design".into()), Some(Access::Edit),
    ).unwrap();
    policy.set_grant(
        "org-admin", "org", "project", Principal::Account("guest".into()), Some(Access::View),
    ).unwrap();
    policy.save(&path).unwrap();
    let mut loaded = OrganizationAccess::load(&path, ["new-admin".into()]).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(loaded.effective_access("lead", "org", "project"), Some(Access::Edit));
    assert_eq!(loaded.effective_access("guest", "org", "project"), Some(Access::View));
    assert_eq!(
        loaded.create_organization("old-admin", "other".into(), "owner".into()),
        Err(AccessError::Forbidden)
    );
    loaded.create_organization("new-admin", "other".into(), "owner".into()).unwrap();
}
