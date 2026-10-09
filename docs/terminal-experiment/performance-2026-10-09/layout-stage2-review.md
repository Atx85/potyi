# Stage 2 output projection review

Candidate: `/private/tmp/potyi-terminal-performance-20261009-111723/candidate`.
Baseline and `layout-stage1` remained unchanged. The review changed only the five allowed candidate files. No builds or tests were run by this worker.

The candidate stores query and published rows in `Arc<Vec<Fragment>>`, and the Pane holds the same immutable frame. Repeated requests for a ready viewport preserve the Arc, String and segment-buffer identities. An unfinished query copies its already bounded frame only when it next appends a completed row while an older publication remains owned. Its existing conservative byte accounting and Vec slack remain in force. The first refresh request schedules projection without cloning a return value. An unchanged frame and grid geometry reuse the existing compatibility grid while updating cursor visibility/position and output-selection paint.

The projection content allowance is unchanged at 256 KiB per query, accounting for Fragment slots, unused Vec capacity, String capacity and segment Vec capacity. This is not a global terminal memory cap. Small Arc control blocks and Vec headers are additional; Flow scratch, tail snapshots, disk read caches, compatibility grids, glyph caches, selections, command queues and transcript files retain their separate existing bounds.

At the peak of an active refresh, the growing viewport query, Layout's prior published frame and the Pane's prior paint frame can be three different allocations. The navigation query adds one more. Therefore one active Layout/Pane set has at most four such bounded projection buffers, or 1 MiB of the accounted capacities. Usually query, published and Pane all share one viewport allocation.

Git Back keeps one saved base view. `Saved` moves the original Layout as well as the Pane frame; it does not keep only one extra Arc. The saved Layout does not poll in the background, and opening further details reuses the current detail rather than nesting Saved snapshots. At the stable method boundary where it is saved, the Pane frame aliases the published frame or the publication is absent, so the saved set has at most two viewport buffers and one navigation buffer (768 KiB). Together with an active refresh this gives a tighter capacity ceiling of 1.75 MiB for these projections. A conservative bound treating both sets independently is 2 MiB. Neither number includes the separately bounded state above or allocator metadata.

Added meaningful tests:

- `shared_view_reuses_ready_frame_without_copying_text_or_segments`: mixed native/plain/Unicode/error rows retain the same ready frame, text and span buffers across repeated polls and requests.
- `growing_shared_frame_preserves_published_anchors_and_byte_budget`: deterministic one-row partial publication grows to four rows without altering the old paint/hit anchors; both frames remain within the existing cap.
- `old_and_new_shared_viewports_each_keep_the_original_content_cap`: wide output reaches the unchanged cap without shrinking the viewport, retains immutable old anchors while scrolling, exercises three distinct viewport buffers plus navigation, and verifies Clear drops Layout-owned publications and queries.

The two Stage 1 differential tests remain. Root should run all layout tests, existing app behavior/keyboard/selection/Git Back checks, and visual/clipboard comparisons; in particular the compatibility-grid reuse needs the existing selection and cursor cases. Root owns compilation and measurement.
