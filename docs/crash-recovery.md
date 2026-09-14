# Crash recovery

Potyi journals edits for the documents open in the editor. On the next launch,
unsaved sessions appear in the command bar. They also remain available after
closing an unsaved document normally.

```
:recover
:recover 1
```

The first command lists available sessions. The second opens the numbered
session in a separate recovered file. Save the current document or switch to
an empty pane first; recovery will not replace unsaved work in that pane.

Use Save As to choose the recovered document's destination. Save keeps the
separate recovered file. Once that succeeds, the old recovery session is
retired. Original files are never overwritten by the recovery command, even
if they changed or were deleted outside Potyi. Closing the recovery panel
leaves the session available. Documents still open in another Potyi process
are excluded using an operating-system file lock.

## Memory and storage

Recovery keeps no second document or piece list in RAM and has no growing
background queue. It adds a small per-document state and uses a 64 KiB copying
buffer and an 8 KiB journal buffer. These buffers are used temporarily.

On the first edit, Potyi streams an immutable backup of the original file to
its recovery directory. This costs disk space proportional to the original
file and may take time for large files. The live piece table then reads that
same immutable backup. Inserted text remains in the existing file-backed edit
store; recovery links to it when both directories are on the same filesystem.
Across filesystems, newly appended text is copied in bounded chunks.

Normal insertions and deletions append small records describing their
positions and piece references. Formatting, bulk replacements and snapshot
undo/redo stream the existing piece list as a single journal frame. They do
not allocate another piece list for journaling. Journal disk usage grows with
operations; document text stays file-backed.

Data is synchronized before the corresponding journal frame is committed.
Frames have lengths and SHA-256 checksums. Saving publishes a completed file
with an atomic replacement, then marks the recovery session saved. On Unix,
the destination directory is synchronized before retiring recovery. These
synchronizations add disk I/O to edits. Recovery does not promise protection
against failed storage hardware or filesystem corruption.

The recovery directory is:

- Windows: `%LOCALAPPDATA%\Potyi\recovery`
- macOS: `~/Library/Application Support/Potyi/recovery`
- Linux: `$XDG_STATE_HOME/potyi/recovery`, otherwise `~/.local/state/potyi/recovery`

Clean sessions are removed on close or during a later recovery listing.
Unsaved sessions remain until the recovered work is saved. Recovered copies
are regular files in this directory; Save As can place the work elsewhere.
Recovery data contains document content and stays local. On Unix, the recovery
directory is private to the user.

## What is recovered

Recovery rebuilds text through the last complete journal frame, including
insertions, deletions, undo/redo results, formatting and bulk replacements.
An interrupted final frame is ignored; the recovery panel reports this.
The undo history, selections, cursor positions and window layout are not
restored. A multi-step edit across several documents can stop between its
individual operations; recovery is per document, not a cross-file transaction.

If recovery cannot write, Potyi keeps the current document editable and displays
a warning to save it. Earlier valid recovery data is preserved. Failed saves
do not clear the recovery session. A damaged session does not hide other
recoverable documents.

## Verification

Automated tests cover a forcibly killed process, live-process exclusion,
truncated journal tails, checksum damage, unavailable storage, cross-filesystem
copying, Unicode, deletions, undo/redo, formatting, Save As and outside changes.
They run in the native Windows, macOS and Linux build workflow.

An isolated piece-table memory check on macOS ARM64 used a 256 MiB original file
and 1,000 insertions. Peak resident memory was 6,213,632 bytes without recovery
and 6,541,312 bytes with recovery: 327,680 bytes (0.3125 MiB) extra. This measures
the test process, not the entire graphical editor. It demonstrates bounded
recovery overhead in that workload, rather than a second 256 MiB allocation.
The raw measurements are recorded in [recovery-memory-results.json](recovery-memory-results.json).
