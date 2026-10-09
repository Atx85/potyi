// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "potyi-save-tests-{}-{}",
            std::process::id(),
            next_document_revision(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn edited(&self, path: &Path) -> PieceTable {
        let mut table = PieceTable::open(path.to_str().unwrap()).unwrap();
        table.insert(0, "edited ").unwrap();
        table
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(unix)]
#[test]
fn save_preserves_executable_and_private_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    for mode in [0o755, 0o600] {
        let path = fixture.0.join(format!("mode-{mode:o}.sh"));
        fs::write(&path, "original\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        fixture.edited(&path).write_to(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "edited original\n");
    }
}

#[cfg(unix)]
#[test]
fn save_follows_relative_symlink_chains_and_preserves_target_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let fixture = Fixture::new();
    let target = fixture.0.join("target.txt");
    let link = fixture.0.join("link.txt");
    let outer = fixture.0.join("outer.txt");
    fs::write(&target, "original\n").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    symlink("target.txt", &link).unwrap();
    symlink("link.txt", &outer).unwrap();
    let table = fixture.edited(&outer);
    table.write_to(&outer).unwrap();
    table.write_to(&outer).unwrap(); // Saving again still reads the open original.
    assert_eq!(fs::read_link(&outer).unwrap(), Path::new("link.txt"));
    assert_eq!(fs::read_link(&link).unwrap(), Path::new("target.txt"));
    assert_eq!(fs::read_to_string(&target).unwrap(), "edited original\n");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 3);
}

#[cfg(unix)]
#[test]
fn save_does_not_replace_dangling_links_or_overwrite_links_for_new_files() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let link = fixture.0.join("link.txt");
    symlink("missing.txt", &link).unwrap();
    let mut table = PieceTable::empty().unwrap();
    table.insert(0, "draft").unwrap();
    assert_eq!(
        table.write_to(&link).unwrap_err().kind(),
        io::ErrorKind::NotFound
    );
    assert_eq!(
        table.write_to_new(&link).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read_link(&link).unwrap(), Path::new("missing.txt"));
    assert!(!fixture.0.join("missing.txt").exists());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn save_as_overwrite_uses_destination_permissions_and_new_files_stay_exclusive() {
    let fixture = Fixture::new();
    let source = fixture.0.join("source.txt");
    let destination = fixture.0.join("destination.txt");
    fs::write(&source, "original\n").unwrap();
    fs::write(&destination, "existing\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o640)).unwrap();
    }
    let permissions = fs::metadata(&destination).unwrap().permissions();
    let table = fixture.edited(&source);
    table.write_to(&destination).unwrap();
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions(),
        permissions
    );
    assert_eq!(fs::read_to_string(&source).unwrap(), "original\n");
    let new = fixture.0.join("new.txt");
    table.write_to_new(&new).unwrap();
    assert_eq!(
        table.write_to_new(&new).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read_to_string(&new).unwrap(), "edited original\n");
}
