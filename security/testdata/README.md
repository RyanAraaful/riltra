# Scanner test fixtures (synthetic)

Every value under `blocked/` is a synthetic string shaped like a local path or
credential; none is real. `blocked/` fixtures must be detected by
`security/scan.sh`; `allowed/` fixtures must not be.

The scanner config allowlists this directory by path so repository scans stay
clean, and `security/scan-tests.sh` copies each fixture to a temporary
directory (outside the allowlist) to prove the rules actually fire.

Do not place real secrets, real usernames, or real home directories here.
