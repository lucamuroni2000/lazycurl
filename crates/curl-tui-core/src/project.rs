use crate::collection::slugify;
use crate::types::Project;
use std::path::{Path, PathBuf};

/// Returns the directory path for a project: `projects_dir/<slug>/`
pub fn project_dir(projects_dir: &Path, project: &Project) -> PathBuf {
    let slug = slugify(&project.name);
    projects_dir.join(slug)
}

/// Find the directory path for a project, handling slug collisions.
/// If a directory already exists with this project's UUID, reuse it.
/// Otherwise try the base slug, then add numeric suffixes.
fn find_dir_for(projects_dir: &Path, project: &Project) -> PathBuf {
    let base_slug = slugify(&project.name);

    // If a dir already exists with this project's id, reuse its path
    if let Ok(entries) = std::fs::read_dir(projects_dir) {
        for entry in entries.flatten() {
            let candidate_dir = entry.path();
            if !candidate_dir.is_dir() {
                continue;
            }
            let manifest = candidate_dir.join("project.json");
            if let Ok(content) = std::fs::read_to_string(&manifest) {
                if let Ok(existing) = serde_json::from_str::<Project>(&content) {
                    if existing.id == project.id {
                        return candidate_dir;
                    }
                }
            }
        }
    }

    // Try base slug
    let candidate = projects_dir.join(&base_slug);
    if !candidate.exists() {
        return candidate;
    }

    // Add numeric suffix
    let mut counter = 2;
    loop {
        let candidate = projects_dir.join(format!("{}-{}", base_slug, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Create a new project directory with `project.json`, `collections/`, and `environments/` subdirs.
/// Returns the created directory path.
/// Rejects empty names/slugs.
pub fn create_project(
    projects_dir: &Path,
    project: &Project,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let slug = slugify(&project.name);
    if project.name.trim().is_empty() || slug.is_empty() {
        return Err("Project name must not be empty".into());
    }

    let dir = find_dir_for(projects_dir, project);

    std::fs::create_dir_all(dir.join("collections"))?;
    std::fs::create_dir_all(dir.join("environments"))?;

    let content = serde_json::to_string_pretty(project)?;
    std::fs::write(dir.join("project.json"), content)?;

    Ok(dir)
}

/// Load a project from a directory (reads `project.json` inside it).
pub fn load_project(project_dir: &Path) -> Result<Project, Box<dyn std::error::Error>> {
    let manifest = project_dir.join("project.json");
    let content = std::fs::read_to_string(&manifest)?;
    let project: Project = serde_json::from_str(&content)?;
    Ok(project)
}

/// List all projects found in `projects_dir`.
/// Returns a vec of `(Project, PathBuf)` where PathBuf is the project directory.
pub fn list_projects(
    projects_dir: &Path,
) -> Result<Vec<(Project, PathBuf)>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();
    if !projects_dir.exists() {
        return Ok(projects);
    }
    for entry in std::fs::read_dir(projects_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            match load_project(&path) {
                Ok(project) => projects.push((project, path)),
                Err(_) => continue, // skip malformed directories
            }
        }
    }
    Ok(projects)
}

/// Delete an entire project directory.
pub fn delete_project(project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::remove_dir_all(project_dir)?;
    Ok(())
}

/// Find a collision-free destination directory for a rename, ignoring the `exclude_dir`.
fn find_rename_dir(projects_dir: &Path, new_slug: &str, exclude_dir: &Path) -> PathBuf {
    // Try base slug
    let candidate = projects_dir.join(new_slug);
    if !candidate.exists() || candidate == exclude_dir {
        return projects_dir.join(new_slug);
    }

    // Add numeric suffix
    let mut counter = 2;
    loop {
        let candidate = projects_dir.join(format!("{}-{}", new_slug, counter));
        if !candidate.exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Rename a project: updates `project.json` and renames the directory if the slug changed.
/// Returns `(new_slug, new_path)`.
pub fn rename_project(
    projects_dir: &Path,
    project: &Project,
    old_slug: &str,
) -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let new_slug = slugify(&project.name);
    if project.name.trim().is_empty() || new_slug.is_empty() {
        return Err("Project name must not be empty".into());
    }

    let old_dir = projects_dir.join(old_slug);
    let new_dir = if new_slug == old_slug {
        // Slug unchanged — update project.json in place
        old_dir.clone()
    } else {
        find_rename_dir(projects_dir, &new_slug, &old_dir)
    };

    let content = serde_json::to_string_pretty(project)?;

    if new_dir != old_dir {
        // Write to old dir, rename dir, then write again to ensure correctness
        std::fs::write(old_dir.join("project.json"), &content)?;
        std::fs::rename(&old_dir, &new_dir)?;
        std::fs::write(new_dir.join("project.json"), &content)?;
    } else {
        std::fs::write(old_dir.join("project.json"), &content)?;
    }

    Ok((new_slug, new_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Project;

    fn make_project(name: &str) -> Project {
        Project {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            active_environment: None,
        }
    }

    #[test]
    fn test_create_and_load_project() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");

        let project = make_project("My API");
        let dir = create_project(&projects_dir, &project).unwrap();

        // Directory and subdirs must exist
        assert!(dir.exists());
        assert!(dir.join("collections").exists());
        assert!(dir.join("environments").exists());
        assert!(dir.join("project.json").exists());

        // Load and verify fields
        let loaded = load_project(&dir).unwrap();
        assert_eq!(loaded.name, "My API");
        assert_eq!(loaded.id, project.id);
        assert!(loaded.active_environment.is_none());
    }

    #[test]
    fn test_list_projects() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");

        create_project(&projects_dir, &make_project("Alpha")).unwrap();
        create_project(&projects_dir, &make_project("Beta")).unwrap();

        let list = list_projects(&projects_dir).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_delete_project() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");

        let project = make_project("Delete Me");
        let dir = create_project(&projects_dir, &project).unwrap();
        assert!(dir.exists());

        delete_project(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_slug_collision() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");

        let p1 = make_project("My Project");
        let p2 = make_project("My Project");

        let dir1 = create_project(&projects_dir, &p1).unwrap();
        let dir2 = create_project(&projects_dir, &p2).unwrap();

        assert_ne!(dir1, dir2);
        assert!(projects_dir.join("my-project").exists());
        assert!(projects_dir.join("my-project-2").exists());
    }

    #[test]
    fn test_empty_name_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");

        let project = make_project("");
        let result = create_project(&projects_dir, &project);
        assert!(result.is_err());

        let project_spaces = make_project("   ");
        let result2 = create_project(&projects_dir, &project_spaces);
        assert!(result2.is_err());
    }

    #[test]
    fn test_project_dir_helper() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path();

        let project = make_project("Hello World");
        let dir = project_dir(projects_dir, &project);
        assert_eq!(dir, projects_dir.join("hello-world"));
    }

    #[test]
    fn test_rename_project() {
        let tmp = tempfile::tempdir().unwrap();
        let projects_dir = tmp.path().join("projects");

        let mut project = make_project("Original Name");
        let _dir = create_project(&projects_dir, &project).unwrap();
        let old_slug = slugify(&project.name);
        assert!(projects_dir.join("original-name").exists());

        // Rename it
        project.name = "New Name".to_string();
        let (new_slug, new_path) = rename_project(&projects_dir, &project, &old_slug).unwrap();

        assert_eq!(new_slug, "new-name");
        assert!(new_path.exists());
        assert!(!projects_dir.join("original-name").exists());

        // Verify project.json has the updated name
        let loaded = load_project(&new_path).unwrap();
        assert_eq!(loaded.name, "New Name");
        assert_eq!(loaded.id, project.id);
    }
}
