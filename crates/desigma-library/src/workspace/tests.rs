use super::*;

#[test]
fn organization_switching_persists_selection_and_separates_projects() {
    let root = std::env::temp_dir().join(format!("desigma-workspace-{}", new_id()));
    let mut workspace = Workspace::open(root.clone()).unwrap();
    Library::open(workspace.active_root()).unwrap().create_project("Local project").unwrap();
    let first = workspace.create("First organization").unwrap();
    first.create_project("First project").unwrap();
    let first_id = workspace.active().unwrap().to_owned();
    let second = workspace.create("Second organization").unwrap();
    assert!(second.list_projects().is_empty());
    assert!(workspace.create("first organization").is_err());
    let mut reopened = Workspace::open(root.clone()).unwrap();
    assert_eq!(reopened.organizations().len(), 2);
    assert_eq!(reopened.active_root(), second.root());
    assert_eq!(reopened.select(Some(first_id)).unwrap().list_projects()[0].name, "First project");
    assert_eq!(reopened.select(None).unwrap().list_projects()[0].name, "Local project");
    fs::remove_dir_all(root).unwrap();
}
