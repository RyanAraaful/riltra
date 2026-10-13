# Riltra — native code intelligence + engineering knowledge graph (M0 bootstrap)

> **Status: M0 — project bootstrap & legal baseline. Belum ada parser, graph,
> MCP, knowledge ingestion, embedding, atau UI.** Semua klaim performa/akurasi
> kompetitor dilarang sampai bukti benchmark reproduktif tersedia (§M10 blueprint).

**Visi:** code intelligence dan persistent engineering knowledge yang lokal,
cepat, transparan, lintas bahasa, callable dari terminal/AI agent, legal untuk
bisnis di bawah Apache-2.0.

**Arsitektur target (belum dibangun):** Rust core + Tree-sitter syntax,
semantic opsional via LSP/SCIP, typed property graph, SQLite + FTS5, satu API
untuk CLI dan MCP stdio. Core-only tetap offline; provider semantik opt-in.

**Nama binary M0:** `riltra`. Blueprint memakai nama kerja `ocik` sebagai contoh
kontrak CLI, **bukan** klaim merek tersedia — M0 tidak mengadopsi `ocik`.

## Scope M0 (kontrak §15/§18 blueprint)

- Rust workspace kosong yang compile; CLI placeholder std-only tanpa networking.
- `LICENSE` Apache-2.0, `NOTICE`, `CONTRIBUTING.md` (DCO), `SECURITY.md`,
  `docs/licensing.md` (inventory dependency + sumber).
- CI lint/format/test Windows + Linux + macOS.
- Fixtures kecil self-authored di `fixtures/`; histori repo bersih.
- **STOP gate:** tanpa kode TK internal / GitNexus; klaim nama belum diverifikasi.

## Non-goals M0

Parser, code graph, freshness/incremental, LSP/SCIP, impact/trace, knowledge
graph, MCP server, embeddings, UI/TUI, SaaS, cross-repo tracing.

## Quickstart

```powershell
cargo fmt --check
cargo check --locked
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo run -q -p riltra -- status
```

Output `status` deterministik (satu baris JSON, key order tetap):

```json
{"name":"riltra","version":"0.1.0","scope":"m1-registry","networking":"disabled"}
```

## M1: repository registry (CLI)

```powershell
cargo run -q -p riltra -- init --repo D:\Projects\sample
cargo run -q -p riltra -- repo list
cargo run -q -p riltra -- repo status --repo sample
cargo run -q -p riltra -- doctor --repo sample
```

- Registry disimpan di `%APPDATA%\riltra\registry.json` (Windows) atau
  `~/.config/riltra/registry.json` (Linux/macOS); override isolasi dengan
  env `RILTRA_HOME` (menunjuk langsung ke direktori registry, dipakai tests).
- `repo status` → `head/branch/dirty/untracked/toplevel/worktree` + info
  `.riltraignore`; repo non-git → `present:false`; `.git` korup atau `git status`
  gagal → `valid:false` + exit 3 (JSON tetap ke stdout; tanpa bukti status,
  tidak ada klaim `clean`/`valid`).
- `repo status` menolak root yang berpindah atau di-retarget via symlink/junction
  (canonical saat ini wajib sama dengan record; selain itu exit 3 tanpa stdout).
- Exit codes: 0 ok, 2 usage error, 3 repository/data error.
- Keamanan path: root dikanonikalisasi saat `init`, nama registry validated
  (tanpa separator/`..`), query selalu lewat record kanonis; symlink-escape
  terdeteksi karena hasil canonicalize keluar root. Git dibaca via plumbing
  read-only (`GIT_TERMINAL_PROMPT=0`, `--no-optional-locks`); kode repo tidak
  pernah dieksekusi. Tanpa `.git` di root → `absent` (tidak meminjam repo induk).
- CLI std-only (tanpa `clap`/`serde`); ditunda sampai permukaan command
  membesar (pra-M8).

Catatan: `0.1.0` adalah versi workspace bootstrap, **bukan** rilis produk
(rilis `v0.1.0` di-gate pada M9 bila MVP benar-benar usable).

## Lisensi

Apache-2.0 (`LICENSE`). Ketergantungan/grammar/provider kelak diaudit per
paket+versi di `docs/licensing.md` sebelum dibundel. Jangan memasukkan kode
berlisensi noncommercial/restricted tanpa izin yang sesuai.
