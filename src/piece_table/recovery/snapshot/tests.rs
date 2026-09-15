// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use crate::piece_table::recovery::tests::Temp;
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
};

#[test]
fn recovery_snapshot_fallback_copies_exact_range_without_moving_source_cursor() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    // Cross multiple fallback buffer boundaries, with an outside append beyond
    // the length known by the piece table.
    let bytes: Vec<u8> = (0..150_000).map(|i| (i % 251) as u8).collect();
    fs::write(&source, &bytes).unwrap();
    let mut input = File::open(&source).unwrap();
    input.seek(SeekFrom::Start(17)).unwrap();
    let destination = temp.0.join("snapshot");
    let mut output = create_with(&input, &destination, 140_000, |_, _| Ok(None)).unwrap();
    assert_eq!(input.stream_position().unwrap(), 17);
    fs::write(&source, "outside change").unwrap();
    output.seek(SeekFrom::Start(0)).unwrap();
    let mut recovered = Vec::new();
    output.read_to_end(&mut recovered).unwrap();
    assert_eq!(recovered, bytes[..140_000]);
}

#[test]
fn recovery_snapshot_restores_source_cursor_after_copy_success_and_failure() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::write(&source, b"0123456789").unwrap();
    let mut input = File::open(&source).unwrap();
    for fail in [false, true] {
        input.seek(SeekFrom::Start(3)).unwrap();
        let result = preserving_source_cursor(&input, || {
            // Exercise a read that changes the shared cursor on every OS,
            // matching the Windows seek_read behavior used by copy_range.
            let mut reader = &input;
            reader.seek(SeekFrom::Start(7))?;
            let mut bytes = [0; 2];
            reader.read_exact(&mut bytes)?;
            assert_eq!(&bytes, b"78");
            if fail { Err(io::Error::other("copy failed")) } else { Ok(()) }
        });
        if fail {
            assert_eq!(result.unwrap_err().to_string(), "copy failed");
        } else {
            result.unwrap();
        }
        let mut next = [0; 2];
        input.read_exact(&mut next).unwrap();
        assert_eq!(&next, b"34");
    }
}

#[test]
fn recovery_snapshot_never_overwrites_an_existing_destination() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    let destination = temp.0.join("snapshot");
    fs::write(&source, "source").unwrap();
    fs::write(&destination, "keep me").unwrap();
    let input = File::open(&source).unwrap();
    assert!(create(&input, &destination, 6).is_err());
    assert_eq!(fs::read_to_string(&destination).unwrap(), "keep me");
}

#[test]
fn recovery_snapshot_rejects_shortened_sources_in_both_paths() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::write(&source, "short").unwrap();
    let mut input = File::open(&source).unwrap();
    input.seek(SeekFrom::Start(2)).unwrap();
    for native in [false, true] {
        let destination = temp.0.join(format!("snapshot-{native}"));
        let result = if native {
            create(&input, &destination, 100)
        } else {
            create_with(&input, &destination, 100, |_, _| Ok(None))
        };
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
        assert_eq!(input.stream_position().unwrap(), 2);
    }
}

#[test]
fn recovery_snapshot_empty_original_skips_clone_and_does_not_copy_appends() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::write(&source, "outside append").unwrap();
    let output = create_with(
        &File::open(source).unwrap(),
        &temp.0.join("snapshot"),
        0,
        |_, _| panic!("empty originals must not be cloned"),
    )
    .unwrap();
    assert_eq!(output.metadata().unwrap().len(), 0);
}

#[test]
fn recovery_snapshot_trims_appends_and_keeps_both_files_independent() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::write(&source, "original plus outside append").unwrap();
    let mut input = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&source)
        .unwrap();
    let destination = temp.0.join("snapshot");
    let mut output = create(&input, &destination, 8).unwrap();
    assert_eq!(
        fs::read_to_string(&source).unwrap(),
        "original plus outside append"
    );
    input.write_all(b"EXTERNAL").unwrap();
    assert_eq!(fs::read_to_string(&destination).unwrap(), "original");
    output.seek(SeekFrom::Start(0)).unwrap();
    output.write_all(b"SNAPSHOT").unwrap();
    assert!(fs::read_to_string(&source).unwrap().starts_with("EXTERNAL"));
}

#[test]
fn recovery_native_clone_is_independent_when_supported() {
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::write(&source, vec![b'a'; 128 * 1024]).unwrap();
    let input = File::open(&source).unwrap();
    let destination = temp.0.join("snapshot");
    let Some(output) = try_clone(&input, &destination).unwrap() else {
        eprintln!(
            "Native clone unavailable on this test filesystem; fallback tests cover copying."
        );
        return;
    };
    output.sync_all().unwrap();
    fs::write(&source, "changed").unwrap();
    assert_eq!(fs::read(&destination).unwrap(), vec![b'a'; 128 * 1024]);
    eprintln!("Native copy-on-write clone exercised successfully.");
}

#[cfg(unix)]
#[test]
fn recovery_snapshot_of_read_only_file_is_private_and_writable() {
    use std::os::unix::fs::PermissionsExt;
    let temp = Temp::new();
    let source = temp.0.join("source");
    fs::write(&source, "read only").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o444)).unwrap();
    let output = create(&File::open(&source).unwrap(), &temp.0.join("snapshot"), 9).unwrap();
    assert_eq!(
        output.metadata().unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(&source).unwrap().permissions().mode() & 0o777,
        0o444
    );
}
