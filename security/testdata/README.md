# Scanner test fixtures (synthetic)

Every value under `blocked/` is a synthetic string shaped like a local path or
credential; none is real. `blocked/` fixtures must be detected by the scanner;
`allowed/` fixtures must not be.

The scanner config allowlists the known blocker fixture files individually, by
exact path. It does not allowlist this directory: a new file placed anywhere
under `security/testdata/` is still scanned, and `security/scan-tests.sh`
proves both the fixture exceptions and that new-file behavior.

Do not place real secrets, real usernames, or real home directories here.
