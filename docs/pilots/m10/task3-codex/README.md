# M10 Task 3 — Codex pilot evidence

The historical precheck record remains in PRECHECK.md. It describes the state before owner GO and still reports zero benchmark sessions for that point in time; later pilot evidence is stored separately.

The authorized eight-session pilot evidence and grading are in [pilot-20261026T162908300Z](pilot-20261026T162908300Z/README.md). The pilot stopped after all eight first-attempt slots because both A4 sessions read operator skill files outside their assigned snapshots. The final review confirms 4 PASS, 2 FAIL, and 2 UNAVAILABLE; it does not characterize the pilot as clean. See the pilot README, `grading.json`, and each A4 `filesystem-adjudication.json`.

The sanitized review archive is [task3-codex-sanitized-20261026T162908300Z.zip](../task3-codex-sanitized-20261026T162908300Z.zip). `SHA256SUMS.txt` covers this directory tree; the pilot directory retains its own manifest. The original raw runset remains in restricted local storage and is not included in the archive. No benchmark rerun, commit, or push was made during final review.

The package-local `.gitattributes` disables line-ending conversion for Task 3 evidence files so committed bytes continue to match both SHA-256 manifests and the review ZIP.
