// Pötyi - SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

fn request(path: String) -> Request {
    Request {
        token: "private-test-token".into(),
        operation: "view".into(),
        directory: PathBuf::from("é folder"),
        path,
        command_id: None,
        exit_status: None,
    }
}

#[test]
fn experimental_terminal_bridge_packet_retains_wire_format_and_encoded_limit() {
    let empty = request_packet(&request(String::new())).unwrap();
    let overhead = empty.len() - 4;
    let packet = request_packet(&request("x".repeat(MAX_REQUEST_BYTES - overhead))).unwrap();
    assert_eq!(packet.len(), MAX_REQUEST_BYTES + 4);
    assert_eq!(
        u32::from_be_bytes(packet[..4].try_into().unwrap()) as usize,
        MAX_REQUEST_BYTES
    );
    let decoded: Request = serde_json::from_slice(&packet[4..]).unwrap();
    assert_eq!(decoded.directory, PathBuf::from("é folder"));
    assert_eq!(decoded.path.len(), MAX_REQUEST_BYTES - overhead);
    assert!(request_packet(&request("x".repeat(MAX_REQUEST_BYTES - overhead + 1))).is_err());
    // These bytes fit before JSON encoding, but their escapes exceed the cap.
    assert!(request_packet(&request("\0".repeat(12_000))).is_err());
}

#[test]
fn experimental_terminal_bridge_accepts_coalesced_and_fragmented_request_frames() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let reader = std::thread::spawn(move || {
        for expected in ["space 東京.rs:3:2", "é notes.rs:4"] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let request = read_request(&mut stream).unwrap();
            assert_eq!(request.path, expected);
            assert_eq!(request.token, "private-test-token");
            stream.write_all(b"OK").unwrap();
        }
    });
    for (path, fragmented) in [("space 東京.rs:3:2", false), ("é notes.rs:4", true)] {
        let packet = request_packet(&request(path.into())).unwrap();
        let mut stream = TcpStream::connect(address).unwrap();
        stream.set_nodelay(true).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        if fragmented {
            for bytes in packet.chunks(3) {
                stream.write_all(bytes).unwrap();
            }
        } else {
            stream.write_all(&packet).unwrap();
        }
        let mut reply = String::new();
        stream.read_to_string(&mut reply).unwrap();
        assert_eq!(reply, "OK");
    }
    reader.join().unwrap();
}

#[test]
fn experimental_terminal_bridge_rejects_oversize_and_stays_available() {
    let bridge = Bridge::new(Arc::new(|| {})).unwrap();
    assert!(
        client(vec![
            bridge.address.clone(),
            bridge.token.clone(),
            "edit".into(),
            "\0".repeat(12_000)
        ])
        .is_err()
    );
    assert!(bridge.requests.try_recv().is_err());
    client(vec![
        bridge.address.clone(),
        bridge.token.clone(),
        "complete".into(),
        "27".into(),
        "19".into(),
    ])
    .unwrap();
    let request = bridge
        .requests
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    assert_eq!(request.command_id, Some(27));
    assert_eq!(request.exit_status, Some(19));
}

#[test]
fn experimental_terminal_bridge_failed_ack_is_not_reported_as_success() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let reader = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let request = read_request(&mut stream).unwrap();
        assert_eq!(request.command_id, Some(9));
        stream.write_all(b"Editor request queue is busy").unwrap();
    });
    let error = client(vec![
        address.to_string(),
        "private-test-token".into(),
        "complete".into(),
        "9".into(),
        "0".into(),
    ])
    .unwrap_err();
    assert_eq!(error.to_string(), "Editor request queue is busy");
    reader.join().unwrap();
}

struct TraceFile(PathBuf);
impl TraceFile {
    fn new() -> Self {
        let mut random = [0; 16];
        getrandom::getrandom(&mut random).unwrap();
        let suffix: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        Self(std::env::temp_dir().join(format!("potyi-helper-trace-{suffix}.log")))
    }
}
impl Drop for TraceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn experimental_terminal_bridge_prefers_sibling_helper_and_falls_back_for_a_copied_app() {
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let unique = TraceFile::new();
    let directory = Directory(unique.0.with_extension("helper é with spaces"));
    std::fs::create_dir(&directory.0).unwrap();
    let application = directory
        .0
        .join(format!("potyi{}", std::env::consts::EXE_SUFFIX));
    let helper = directory
        .0
        .join(format!("potyi-term-helper{}", std::env::consts::EXE_SUFFIX));

    assert_eq!(client_executable(&application), application);
    std::fs::create_dir(&helper).unwrap();
    assert_eq!(client_executable(&application), application);
    std::fs::remove_dir(&helper).unwrap();
    std::fs::write(&helper, b"helper fixture").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(client_executable(&application), application);
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    assert_eq!(client_executable(&application), helper);
    std::fs::remove_file(&helper).unwrap();
    assert_eq!(client_executable(&application), application);
}

#[test]
fn experimental_terminal_bridge_trace_is_bounded_and_sanitizes_operation() {
    assert!(ClientTrace::open(Path::new("relative-trace.log")).is_none());
    let path = TraceFile::new();
    let trace = ClientTrace::open(&path.0).unwrap();
    trace.record("secret-token\nprivate command path", "client_enter");
    for _ in 0..2_048 {
        trace.record("complete", "completed");
    }
    let bytes = std::fs::read(&path.0).unwrap();
    assert!(bytes.len() as u64 <= MAX_TRACE_BYTES);
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("operation=invalid stage=client_enter"));
    assert!(!text.contains("secret-token"));
    assert!(!text.contains("private command path"));
    assert!(text.ends_with('\n'));
    assert!(text.lines().count() > 1);
    let capped_size = text.len();
    trace.record("view", "sent");
    assert_eq!(
        std::fs::metadata(&path.0).unwrap().len(),
        capped_size as u64
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path.0).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn experimental_terminal_bridge_trace_skips_contention_and_preserves_records() {
    let path = TraceFile::new();
    let trace = ClientTrace::open(&path.0).unwrap();
    let locked_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path.0)
        .unwrap();
    locked_file.try_lock().unwrap();
    trace.record("ready", "sent");
    assert_eq!(std::fs::metadata(&path.0).unwrap().len(), 0);
    locked_file.unlock().unwrap();
    drop(locked_file);
    drop(trace);
    let writers: Vec<_> = (0..8)
        .map(|_| {
            let path = path.0.clone();
            std::thread::spawn(move || {
                let trace = ClientTrace::open(&path).unwrap();
                for _ in 0..512 {
                    trace.record("complete", "replied");
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let text = std::fs::read_to_string(&path.0).unwrap();
    assert!(!text.is_empty());
    assert!(text.len() as u64 <= MAX_TRACE_BYTES);
    assert!(text.ends_with('\n'));
    for record in text.lines() {
        assert!(record.starts_with("POTYI_TERM_TRACE pid="));
        assert!(record.contains(" operation=complete stage=replied elapsed_us="));
        assert!(record.rsplit_once('=').unwrap().1.parse::<u128>().is_ok());
    }
}

#[cfg(unix)]
#[test]
fn experimental_terminal_ksh_helper_cd_uses_the_builtin_and_reports_real_cwd() {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let Some(shell) = ["/bin/ksh", "/usr/bin/ksh", "/usr/local/bin/ksh"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
    else {
        return;
    };
    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let unique = TraceFile::new();
    let directory = Directory(unique.0.with_extension("helpers"));
    let target = directory.0.join("cwd é ' with spaces");
    std::fs::create_dir_all(&target).unwrap();
    let target = target.canonicalize().unwrap();
    let helper = directory.0.join("cwd-helper");
    std::fs::write(&helper, "#!/bin/sh\n[ \"$4\" = cwd ] || exit 91\nprintf '%s' \"$PWD\" > \"$POTYI_CWD_TEST_RESULT\"\n").unwrap();
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700)).unwrap();
    let result = directory.0.join("reported-cwd");
    let quoted = format!("'{}'", target.to_str().unwrap().replace('\'', "'\\''"));
    let output = Command::new(shell)
        .arg("-c")
        .arg(format!(
            "{}\ncd {quoted} || exit $?\nprintf '%s' \"$PWD\"",
            setup_helpers("ksh")
        ))
        .current_dir(&directory.0)
        .env("POTYI_TERM_CLIENT", helper)
        .env("POTYI_TERM_ADDRESS", "test-address")
        .env("POTYI_TERM_TOKEN", "test-token")
        .env("POTYI_CWD_TEST_RESULT", &result)
        .env_remove("ENV")
        .env_remove("BASH_ENV")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, target.to_str().unwrap().as_bytes());
    assert_eq!(std::fs::read(result).unwrap(), output.stdout);
}
