// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

#[test]
fn experimental_terminal_cmd_cwd_preserves_native_spelling_and_only_removes_disk_prefix() {
    for local in [
        r"c:\Project é 界 with spaces",
        r"D:\",
        r"C:\folder\NULx\COM10",
    ] {
        assert_eq!(
            cmd_directory(Path::new(local)).unwrap(),
            PathBuf::from(local)
        );
        assert_eq!(
            cmd_directory(Path::new(&format!(r"\\?\{local}"))).unwrap(),
            PathBuf::from(local)
        );
    }
    let mut native: Vec<u16> = r"c:\native ".encode_utf16().collect();
    native.extend([0xd800, b'x' as u16]);
    let mut extended: Vec<u16> = r"\\?\".encode_utf16().collect();
    extended.extend(&native);
    let extended = PathBuf::from(OsString::from_wide(&extended));
    assert_eq!(
        cmd_directory(&extended)
            .unwrap()
            .as_os_str()
            .encode_wide()
            .collect::<Vec<_>>(),
        native
    );
}

#[test]
fn experimental_terminal_cmd_cwd_rejects_paths_that_could_change_directory_identity() {
    for path in [
        r"\\server\share\folder",
        r"\\?\UNC\server\share\folder",
        r"\\?\Volume{guid}\folder",
        r"\\.\C:\folder",
        r"C:relative",
        r"\root-relative",
        r"\\?\C:\folder.\child",
        r"\\?\C:\folder \child",
        r"\\?\C:\NUL\child",
        r"\\?\C:\aux.txt\child",
        r"\\?\C:\CoM1\child",
        r"\\?\C:\LPT²\child",
        r"\\?\C:\name/child",
        r"\\?\C:\name:stream",
    ] {
        assert_eq!(
            cmd_directory(Path::new(path)).unwrap_err().kind(),
            io::ErrorKind::Unsupported,
            "{path}"
        );
    }
    let long = format!(r"\\?\C:\{}\{}", "x".repeat(150), "y".repeat(110));
    assert_eq!(
        cmd_directory(Path::new(&long)).unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
}
