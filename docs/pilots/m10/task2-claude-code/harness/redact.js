// SPDX-License-Identifier: Apache-2.0
// Redacts machine-local paths and identifiers in one text file, in five passes:
//   1. literal, longest-first replacement of known roots (scratch, repository, home, scratchpad),
//      the path-derived project folder names used by the client, and the account name;
//   2. the rest of any path that follows a placeholder: separators become "/";
//      the Windows verbatim prefix; the local named-pipe address (raw or escaped);
//      the local PowerShell path;
//   3. any other local machine path, in POSIX drive form or Windows drive form, up to the end of
//      its last segment;
//   4. a generic pass for any drive-letter root left over.
// Usage: node redact.js <in file> <out file> ; prints "label<TAB>count" for each label that changed.
// Inputs come from the environment (set by the caller, never written into this file):
//   ROOT_WIN ROOT_POSIX REPO_WIN REPO_POSIX HOME_WIN HOME_POSIX HOME_SHORT_WIN HOME_SHORT_POSIX
//   SCRATCHPAD_WIN SCRATCHPAD_POSIX PATH_SLUGS (comma-separated) USER_NAME [RUN_EMAIL]
// Git Bash hands POSIX-looking variables to node in Windows form, so every root is matched in its
// Windows form, its POSIX form, and their escaped forms (single, double, and quadruple backslashes).
const fs = require("fs");

const [src, dst] = process.argv.slice(2);
const env = process.env;
const toPosix = (w) => w.replace(/^([A-Za-z]):/, (m, d) => "/" + d.toLowerCase()).replace(/\\/g, "/");
const pairs = [];
function add(label, original, token) {
  if (!original) return;
  const forms = new Set([
    original,
    original.replace(/\\/g, "\\\\"),
    original.replace(/\\/g, "\\\\\\\\"),
    original.replace(/\\/g, "/"),
    toPosix(original),
  ]);
  for (const from of forms) pairs.push({ label, from, token });
}
add("scratchpad", env.SCRATCHPAD_WIN, "<SCRATCHPAD>");
add("scratchpad", env.SCRATCHPAD_POSIX, "<SCRATCHPAD>");
add("scratch", env.ROOT_WIN, "<SCRATCH>");
add("scratch", env.ROOT_POSIX, "<SCRATCH>");
add("repo", env.REPO_WIN, "<REPO>");
add("repo", env.REPO_POSIX, "<REPO>");
add("home", env.HOME_WIN, "<HOME>");
add("home", env.HOME_SHORT_WIN, "<HOME>");
add("home", env.HOME_POSIX, "<HOME>");
add("home", env.HOME_SHORT_POSIX, "<HOME>");
add("user-name", env.USER_NAME, "<USER>");
for (const slug of (env.PATH_SLUGS ?? "").split(",").filter(Boolean)) {
  pairs.push({ label: "project-slug", from: slug, token: "<PROJECT-SLUG>" });
}
if (env.RUN_EMAIL) pairs.push({ label: "identifier", from: env.RUN_EMAIL, token: "<EMAIL>" });
pairs.sort((a, b) => b.from.length - a.from.length);

let text = fs.readFileSync(src, "utf8");
const counts = new Map();
const note = (label, n) => { if (n) counts.set(label, (counts.get(label) ?? 0) + n); };
for (const { label, from, token } of pairs) {
  const parts = text.split(from);
  if (parts.length > 1) {
    note(label, parts.length - 1);
    text = parts.join(token);
  }
}

// Pass 2.
let n = 0;
text = text.replace(/\\+\?\\+/g, () => { n += 1; return ""; });        // verbatim prefix of long paths
note("verbatim-prefix", n);
n = 0;
text = text.replace(/<(?:SCRATCHPAD|SCRATCH|REPO|HOME|PROJECT-SLUG)>[^\s"']*/g, (m) => {
  if (!/\\/.test(m)) return m;
  n += 1;
  return m.replace(/\\+/g, "/");
});
note("placeholder-tail-separator", n);
n = 0;
text = text.replace(/\\+\.\\+pipe\\+[^"'\s,;)]*/g, () => { n += 1; return "<LOCAL-PIPE>"; });   // local named-pipe address
note("local-pipe", n);
n = 0;
text = text.replace(/"messaging_socket_path"\s*:\s*"[^"]*"/g, () => { n += 1; return '"messaging_socket_path": "<LOCAL-PIPE>"'; });
note("local-pipe-field", n);
n = 0;
text = text.replace(/"powershell_path"\s*:\s*"[^"]*"/g, () => { n += 1; return '"powershell_path": "<POWERSHELL-PATH>"'; });
note("powershell-path", n);

// Pass 3. Other local machine paths. A segment may contain spaces only when another segment follows.
n = 0;
text = text.replace(/(\\[nt]|^|[^A-Za-z0-9_\/+])\/[a-z](?:\/[A-Za-z0-9_.+~()\- ]*[A-Za-z0-9_.+~()\-])+/g, (m, pre) => { n += 1; return pre + "<LOCAL-PATH>"; });
note("posix-local-path", n);
n = 0;
text = text.replace(/(?<=\\[nt]|[^A-Za-z0-9_]|^)[A-Za-z]:(?:[\\/]+[A-Za-z0-9_.+~()\- ]*[A-Za-z0-9_.+~()\-])+/g, () => { n += 1; return "<LOCAL-PATH>"; });
note("windows-local-path", n);

// Pass 4. Any drive-letter root left over.
n = 0;
text = text.replace(/(?<=\\[nt]|[^A-Za-z0-9_]|^)[A-Za-z]:(?=[\\/])/g, () => { n += 1; return "<DRIVE>"; });
note("drive-path", n);

fs.writeFileSync(dst, text);
for (const [label, count] of counts) console.log(`${label}\t${count}`);
