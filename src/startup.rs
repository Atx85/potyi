//! Resolve launch paths before loading project configuration or opening documents.

use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LaunchTarget {
    Empty,
    Folder(PathBuf),
    File {
        path: String,
        location: Option<(usize, Option<usize>)>,
    },
}

impl LaunchTarget {
    pub(crate) fn resolve(argument: Option<&str>, cwd: &Path) -> io::Result<Self> {
        let Some(argument) = argument else {
            return Ok(Self::Empty);
        };
        let requested = cwd.join(argument);
        // Existing paths take precedence over :line[:column] suffixes: folder
        // and file names can themselves contain colons on Unix.
        match requested.metadata() {
            Ok(metadata) if metadata.is_dir() => {
                return Ok(Self::Folder(requested.canonicalize()?));
            }
            Ok(_) => {
                return Ok(Self::File {
                    path: argument.into(),
                    location: None,
                });
            }
            // Windows rejects a colon followed by a location suffix as an
            // invalid filename before we get a chance to parse `file:line`.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::InvalidFilename
                ) => {}
            Err(error) => return Err(error),
        }
        let (path, location) = match crate::parse_location(argument) {
            Some((path, line, column)) => (path, Some((line, column))),
            None => (argument, None),
        };
        // Report a missing path at startup, before creating the window.
        if cwd.join(path).metadata()?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Line and column locations require a file, not a folder",
            ));
        }
        Ok(Self::File {
            path: path.into(),
            location,
        })
    }

    pub(crate) fn workspace_root(&self) -> Option<&Path> {
        match self {
            Self::Folder(root) => Some(root),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "potyi-startup-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir(&root).unwrap();
            fs::create_dir(root.join("project with spaces")).unwrap();
            fs::write(root.join("sample.rs"), "one\ntwo\nthree\n").unwrap();
            Self(root.canonicalize().unwrap())
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn folder_launches_resolve_against_the_calling_directory() {
        let fixture = Fixture::new();
        for argument in [".", "project with spaces", "project with spaces/.."] {
            assert_eq!(
                LaunchTarget::resolve(Some(argument), &fixture.0).unwrap(),
                LaunchTarget::Folder(fixture.0.join(argument).canonicalize().unwrap())
            );
        }
        let absolute = fixture.0.join("project with spaces");
        assert_eq!(
            LaunchTarget::resolve(absolute.to_str(), &fixture.0).unwrap(),
            LaunchTarget::Folder(absolute)
        );
        assert_eq!(
            LaunchTarget::resolve(None, &fixture.0).unwrap(),
            LaunchTarget::Empty
        );
    }

    #[test]
    fn file_launches_preserve_location_arguments_and_reject_missing_paths() {
        let fixture = Fixture::new();
        for (argument, location) in [
            ("sample.rs", None),
            ("sample.rs:2", Some((2, None))),
            ("sample.rs:3:1", Some((3, Some(1)))),
        ] {
            assert_eq!(
                LaunchTarget::resolve(Some(argument), &fixture.0).unwrap(),
                LaunchTarget::File {
                    path: "sample.rs".into(),
                    location
                }
            );
        }
        assert_eq!(
            LaunchTarget::resolve(Some("missing"), &fixture.0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            LaunchTarget::resolve(Some("project with spaces:2"), &fixture.0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[cfg(unix)]
    #[test]
    fn opening_a_directory_as_text_preserves_the_current_document() {
        let fixture = Fixture::new();
        let mut editor = crate::Editor::new(crate::config::EditorConfig::default()).unwrap();
        editor.insert_text("unsaved work").unwrap();
        let revision = editor.document.revision();
        let undo_steps = editor.undo_stack.len();
        let folder = fixture.0.join("project with spaces");
        let error = editor.open(folder.to_str().unwrap()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::IsADirectory);
        assert!(error.to_string().contains(folder.to_str().unwrap()));
        assert_eq!(editor.document.text().unwrap(), "unsaved work");
        assert_eq!(editor.document.revision(), revision);
        assert_eq!(editor.undo_stack.len(), undo_steps);
        assert!(editor.dirty);
        assert!(editor.path.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn existing_colon_names_and_symlinked_folders_are_supported() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("project:2")).unwrap();
        fs::write(fixture.0.join("file:2"), "text").unwrap();
        std::os::unix::fs::symlink("project with spaces", fixture.0.join("linked")).unwrap();
        assert_eq!(
            LaunchTarget::resolve(Some("project:2"), &fixture.0).unwrap(),
            LaunchTarget::Folder(fixture.0.join("project:2"))
        );
        assert_eq!(
            LaunchTarget::resolve(Some("file:2"), &fixture.0).unwrap(),
            LaunchTarget::File {
                path: "file:2".into(),
                location: None
            }
        );
        assert_eq!(
            LaunchTarget::resolve(Some("linked"), &fixture.0).unwrap(),
            LaunchTarget::Folder(fixture.0.join("project with spaces"))
        );
    }
}
