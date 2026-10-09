# Remaining startup overhead: diagnostic only

A six-command standalone zsh PTY used the exact frozen lifecycle and full-app
helper, a local mock ACK server and separate builtin timestamp logging. Later
instrumented driver commands took 88.9–101.8 ms. Time from invoking the helper
through receiving its request had a 32.2 ms median. Individual
stty calls took roughly 5–7 ms, with additional file utilities, shell startup and
scheduling. There is no measured single 100 ms terminal-mode restoration wait.

This experiment adds tracing writes, omits the application reader, SDL and layout,
and uses a mock response server. Its timings must not replace the paired acceptance
measurements. It narrows the remaining work to process launch/driver operations
and application settlement rather than claiming TCP framing fixed first use.
Helper-entry trace timings exclude loading the executable. The first-use outlier
is unresolved; this probe does not prove its cause.

Exact events, command wall times, helper/lifecycle/bridge hashes and limitations
are in [results.json](results.json). The temporary generated driver contained an
expired session nonce and is not retained; the extraction/instrumentation script
is retained in the parent directory. No product code changed for this diagnosis.
