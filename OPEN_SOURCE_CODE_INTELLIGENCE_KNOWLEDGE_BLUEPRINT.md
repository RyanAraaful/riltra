# Blueprint Proyek Open-Source: Native Code Intelligence + Knowledge Graph

**Status:** Rekomendasi arsitektur dan Product/Engineering Goals — **belum diimplementasikan**  
**Tanggal evaluasi:** 14 Oktober 2026  
**Nama produk:** Belum diputuskan; contoh CLI dalam dokumen menggunakan nama kerja `ocik` (*Open Code Intelligence & Knowledge*), **bukan klaim nama/merek tersedia**.  
**Lisensi proyek yang direkomendasikan:** Apache License 2.0  
**Platform target:** Windows, Linux, macOS  
**Kanal penggunaan:** CLI native dan MCP server lokal  
**Pemilik keputusan:** Pengelola proyek open-source baru

> **Pembatas utama:** Proyek ini **berdiri sendiri** dan **bukan Terapetra Knowledge (TK)**. TK tetap alat internal. Tidak ada instruksi migrasi, refactor, penyalinan kode, penggantian dependency, atau perubahan repository TK. GitNexus adalah produk pembanding berdasarkan kebutuhan/fungsi publik, **bukan codebase untuk disalin**.

---

## 1. Executive decision

**Keputusan utama:** Bangun **native Rust executable** dengan inti yang menyatukan **syntax index dari Tree-sitter**, **semantic facts dari provider LSP dan/atau SCIP**, **typed property graph**, **SQLite + FTS5** untuk storage/pencarian lokal, serta **CLI dan MCP** yang menggunakan API aplikasi yang sama.

**Keputusan untuk knowledge:** **Tidak memakai Obsidian** sebagai dependency ataupun sumber kebenaran. Gunakan **knowledge graph terstruktur dengan SQLite sebagai canonical store untuk catatan yang dibuat secara native**, dilengkapi riwayat perubahan, bukti, dan ekspor/impor terbuka (`Markdown`/`JSONL`). Dokumen yang diimpor tetap menyimpan referensi sumber aslinya; jangan mengubah fakta dokumen menjadi klaim baru tanpa bukti.

**Keputusan lisensi:** Publikasikan kode yang ditulis sendiri di bawah **Apache-2.0**. Izinkan penggunaan pribadi, organisasi, komersial, fork, dan distribusi sesuai kewajiban lisensi. Semua dependensi, grammar, binary indexer, dan aset tetap membutuhkan pemeriksaan lisensi masing-masing.

**Keputusan penggunaan native:** Inti produk didistribusikan sebagai binary native dan dapat menjalankan syntax search, code graph dasar, knowledge graph, CLI, serta MCP **tanpa Node/Python/JVM service wajib**. Analisis semantik mendalam per bahasa bersifat **opsional** dan dapat membutuhkan indexer/language server eksternal dengan runtime/toolchain masing-masing. Jangan menyamakan “native core” dengan “semua semantic provider menjadi satu binary”.

**Keputusan prioritas:** Correctness → source freshness → selective/incremental indexing → retrieval/token efficiency → semantic coverage → advanced graph capabilities. Target **lebih baik dari GitNexus pada metrik dan workflow yang didefinisikan**, bukan mengklaim lebih baik pada semua fitur tanpa pengujian.

### 1.1 Alasan memilih pendekatan ini

| Alternatif | Nilai utama | Kendala untuk proyek baru | Keputusan |
|---|---|---|---|
| TypeScript/Node.js core | Integrasi ecosystem JS cepat | Runtime Node sebagai bagian utama deployment; bukan alasan untuk unggul secara teknis | Tidak dipilih untuk core |
| Python core | Produktivitas tinggi, ekosistem tooling luas | Tidak sejalan dengan preferensi distribusi binary native sederhana; loop berat perlu profiling | Cocok bagi tooling/eksperimen, bukan core rilis |
| Go core | Native dan sederhana | Integrasi parser/SCIP Rust/Tree-sitter dan beberapa algoritma graph bukan pembeda signifikan | Layak, tetapi pilihan kedua |
| C++ core | Performa/native dan cocok LLVM | Biaya memory-safety, build, dan maintenance lebih tinggi | Tidak dipilih sebagai orchestrator |
| **Rust core + Tree-sitter + provider semantik** | Native, keamanan memori, distribusi lokal, API lintas CLI/MCP | Kompleksitas adapter provider; semantic fidelity bergantung pada tooling bahasa | **Dipilih** |
| Graph DB server sebagai syarat awal | Query graph deklaratif | Deployment dan maintenance bertambah sebelum skala terbukti | **Ditunda** |

**Catatan bukti:** Bahasa implementasi bukan ukuran otomatis kecepatan ataupun akurasi. Rust dipilih untuk karakteristik deployment dan engineering, bukan karena telah terbukti melampaui GitNexus dalam benchmark yang belum dilaksanakan.

---

## 2. Misi produk, persona, dan batasan

### 2.1 Misi

Menyediakan **code intelligence dan persistent engineering knowledge** yang lokal, cepat, transparan, lintas bahasa, dapat dipanggil dari terminal ataupun AI coding agent, dan legal dipakai dalam aktivitas bisnis melalui lisensi open-source permisif.

### 2.2 Pengguna utama

- **Developer individu:** mencari definisi, pemanggil, referensi, dependencies, dan dampak perubahan tanpa membaca keseluruhan repo.
- **Maintainer open-source:** meninjau refactor, perubahan API, serta alasan teknis sebelum menerima PR.
- **Tim engineering:** menyimpan ADR, batasan, keputusan, eksperimen dan hubungannya dengan implementasi saat ini.
- **AI coding agent:** menerima konteks ringkas, evidence-backed, bukan dump ribuan baris yang menghabiskan token.
- **Bisnis:** dapat mengadopsi, mengintegrasikan, memfork, dan mendistribusikan perangkat lunak sesuai Apache-2.0 tanpa larangan penggunaan komersial.

### 2.3 Non-goals awal

- Bukan fork atau turunan kode GitNexus; **tidak mengimpor** source, skema privat, fixture, test, prompt, atau algoritme internal GitNexus yang hak penggunaannya tidak pasti.
- Bukan pengganti IDE/compiler, code execution sandbox, antivirus, atau static security analyzer lengkap.
- Tidak menjanjikan semua bahasa memiliki accuracy yang sama.
- Tidak memerlukan akun/cloud, telemetry, embeddings, model LLM, Obsidian, Neo4j, atau daemon wajib.
- Tidak membuat perubahan source otomatis, menjalankan build script, menginstal package, atau menjalankan LSP provider tanpa izin eksplisit pengguna.
- Tidak mengembangkan UI graph, hosted SaaS, team collaboration, maupun cross-repo semantic tracing pada MVP.
- Tidak membaca/mengakses repository TK internal sebagai bahan implementasi atau pengujian otomatis tanpa keputusan terpisah.

### 2.4 Definisi “melampaui GitNexus”

Klaim **outperform** hanya diperbolehkan untuk scope yang diuji dengan metode reproduktif. Minimal harus ada hasil pada **akurasi relasi**, **freshness perubahan source**, **incremental refresh**, **efisiensi token/tugas**, serta **workflow code + knowledge**. Model lisensi dan distribusi yang lebih terbuka adalah keunggulan produk secara legal/operasional, **bukan bukti performa lebih unggul**.

---

## 3. Stack teknologi pilihan

| Layer | Teknologi | Status keputusan | Mengapa |
|---|---|---|---|
| Core executable | **Rust** | Final target | Native, satu codebase CLI/MCP, kontrol resource |
| CLI | `clap` + `serde`/`serde_json` (versi dan lisensi diperiksa saat lock) | Pilihan implementasi | Command, JSON stable output |
| MCP | **`rmcp`** SDK resmi Rust MCP, stdio pertama | Pilihan utama | Integrasi agent langsung; transport dipisah dari logic |
| Runtime async | **Tokio** jika perlu untuk MCP/provider process | Kondisional | Proses concurrent dan cancellation; hindari worker berlebih |
| Syntax parsing | **Tree-sitter + grammar yang dipin** | Final target | Structural parser lokal dan incremental parsing file |
| Semantic input A | **LSP client adapters** (definition/references/symbols, bila tersedia) | Provider opsional | Membuka ekosistem lintas bahasa |
| Semantic input B | **SCIP reader** (`index.scip`/protobuf) | Provider opsional | Identitas simbol dan relasi presisi dari indexer |
| C++/CUDA provider | **clangd/LLVM** (compile commands) | Prioritas pilot | Memanfaatkan compiler-aware tooling |
| Git state | **`gix`/gitoxide**, validasi dengan Git CLI untuk kasus batas | Pilihan utama | Cross-platform Rust library, lisensi permisif |
| Persistent DB | **SQLite** (`rusqlite`, bundled SQLite bila cocok) | Pilihan utama | Embedded/ACID/portable, tanpa server |
| Full-text search | **SQLite FTS5** | Default | BM25/full-text di dalam DB |
| Graph traversal | Relational edge tables, SQL recursive CTE; **petgraph** hanya bila algoritma memerlukan | Default | Tidak menambah server graph sebelum diperlukan |
| Knowledge canonical | SQLite structured records + immutable evidence refs + revision journal | Final target | Tanpa ketergantungan aplikasi catatan |
| Exchange format | Markdown/JSONL + machine-readable metadata | Final target | Portabilitas dan backup |
| Embeddings/vector | Tidak default; adapter opsional di milestone lanjut | Ditunda | Ketepatan dan biaya retrieval harus dibuktikan |
| UI | Tidak ada di MVP; TUI/visualisasi opsional | Ditunda | CLI/MCP lebih penting |

**Bukti rujukan lisensi primer** tersedia pada §19. `rmcp` per Oktober 2026 menyertakan transisi lisensi MIT→Apache-2.0; simpan notice yang benar untuk versi konkret. Setiap grammar Tree-sitter dan semantic provider dievaluasi **per paket**, bukan diasumsikan mengikuti lisensi engine Tree-sitter.

---

## 4. Arsitektur sistem (modular monolith native)

### 4.1 Diagram tingkat tinggi

```text
                Developer / CI / AI Agent
                        |        |
                     CLI          MCP (stdio)
                        \        /
                      Native Rust Host
                             |
                      Application API
             +---------------+---------------+
             |               |               |
         Repo/Git        Query Planner    Knowledge API
         Freshness       + Token Budget    + Provenance
             |               |               |
    +--------+----------+    |    +---------+------------+
    |                   |    |    |                      |
Syntax Provider    Semantic Providers     Knowledge Ingest
 Tree-sitter       LSP / SCIP import      CLI / MD / JSONL
    |                   |                   |
    +------ normalized, evidence-backed facts --------+
                             |
                 Graph / Search / Ranking
                             |
                    SQLite + FTS5
               (code index dan knowledge)
                             |
             Atomic revisions + source anchors
```

### 4.2 Repo modul yang disarankan

```text
project-root/
  Cargo.toml                 # Rust workspace
  LICENSE                    # Apache-2.0
  NOTICE                     # jika diperlukan
  README.md
  CONTRIBUTING.md
  SECURITY.md
  CODE_OF_CONDUCT.md
  docs/
    architecture.md
    data-contract.md
    mcp-contract.md
    language-support.md
    benchmarks.md
    licensing.md
    goals.md
  crates/
    core/                    # shared API, error, result types
    cli/                     # CLI commands
    mcp/                     # stdio MCP adapter
    repo/                    # registration, git state, worktrees
    parser/                  # tree-sitter and captures
    providers/               # LSP and SCIP adapters
    graph/                   # normalized entities and traversal
    search/                  # symbol, FTS, ranking, context budget
    storage/                 # SQLite, schema, transactions
    knowledge/               # knowledge ingestion, edges, revisions
    evaluation/              # benchmark harness & metrics
  grammars/                  # manifest references/pinned hashes; no blind vendoring
  fixtures/                  # self-authored or clearly licensed fixtures
  tests/
    integration/
    regression/
    security/
    performance/
  .github/workflows/
```

**Prinsip modular monolith:** buat satu workspace Rust, bukan banyak microservice. Modul code dan knowledge punya tanggung jawab masing-masing; CLI dan MCP tidak boleh memiliki dua implementasi query yang berbeda.

### 4.3 Native tidak sama dengan bebas seluruh dependency runtime

- **Bisa selalu native:** executable utama, Tree-sitter grammar yang dibundel secara sah, repository index, SQLite, FTS, knowledge, basic code search, MCP stdio.
- **Bisa memerlukan program luar:** clangd, rust-analyzer, TypeScript/JavaScript language server, indexer SCIP Python, JDT LS, gopls, dsb.
- **Implikasi:** sebut setiap bahasa dengan label `syntax`, `semantic-via-provider`, atau `unsupported`; provider yang tidak terpasang tidak boleh menyebabkan core gagal total.
- **Distribusi:** `core-only` harus tetap berguna dan sepenuhnya offline; `provider-enhanced` memberi akurasi lebih tinggi jika toolchain yang sesuai sudah tersedia.

---

## 5. Desain code intelligence lintas bahasa

### 5.1 Tiga tingkat kemampuan

**Tier S0 — Syntax (tanpa compiler/build):** definisi fungsi/class, import, deklarasi, local call candidates, location, basic scope. Tree-sitter Query/captures; bisa memberikan `candidate` atau `lower_bound`, **bukan jaminan resolve lintas file**.

**Tier S1 — Semantic via LSP:** definition, references, document/workspace symbols, type/documentation yang didukung provider; kemampuan nyata bervariasi menurut language server dan konfigurasi proyek. LSP adalah protokol interaktif, bukan jaminan index lengkap atau portable index dump.

**Tier S2 — Semantic via SCIP:** ingesti fakta dari producer SCIP saat tersedia; gunakan symbol identity yang stabil dalam ruang indeks producer dan occurrence lintas file. SCIP adalah **format**, bukan alat yang otomatis melakukan semantic analysis. Jangan membangun ulang TypeScript/Python/C++ compiler front-end sendiri hanya demi mendukung SCIP.

### 5.2 Prioritas bahasa

| Bahasa | Syntax first | Semantic route yang diusulkan | Catatan validasi |
|---|---|---|---|
| Python | Tree-sitter Python | SCIP Python atau LSP Python | Dynamic calls/imports bisa unresolved; indexer mungkin perlu Node/Python environment |
| TypeScript/JavaScript | Tree-sitter TS/JS | SCIP TypeScript atau TS language server | `tsconfig`, aliases, monorepo, generated types; SCIP TS membutuhkan Node sesuai versinya |
| Rust | Tree-sitter Rust | rust-analyzer via LSP | Macro, trait dispatch, workspace features harus diuji |
| C/C++ | Tree-sitter C/C++ | clangd via LSP + compilation database | Headers, templates, macros, compile flags, platform-specific defines |
| CUDA | C/C++ structural parsing sebatas grammar | clangd/Clang bila konfigurasi CUDA sah | Kernel calls, host/device symbols, compile database; jangan klaim full CUDA parity sebelum uji |
| Go | Tree-sitter Go | gopls via LSP | Modules, build tags; provider harus di-onboard |
| Java | Tree-sitter Java | JDT LS atau SCIP producer yang terverifikasi | JVM/tooling eksternal; bukan core-only |
| C# | Tree-sitter C# | SCIP .NET/Roslyn atau LSP | SDK .NET eksternal, target framework dan project loading |

**MVP product-grade:** Python + TypeScript/JavaScript + Rust + C/C++ sebagai pilot; CUDA **eksperimental** dan tidak boleh menahan rilis awal apabila bukti dukungan semantik belum cukup. Go, Java, C# sebagai perluasan bertahap setelah fondasi lulus. Jumlah bahasa yang diparse jangan dipasarkan sebagai jumlah bahasa beranalisis semantik penuh.

### 5.3 Provider capabilities registry

Setiap provider terdaftar dengan metadata:

```json
{
  "language": "typescript",
  "provider": "scip-typescript",
  "provider_mode": "optional-external",
  "capabilities": ["definitions", "references", "cross_file_symbols"],
  "execution_policy": "explicit-opt-in",
  "state": "available|missing|invalid|unsupported",
  "precision": "semantic",
  "source_revision": "<git-tree-or-content-fingerprint>"
}
```

**Penting:** Contoh di atas adalah kontrak usulan, bukan klaim bahwa provider sudah terpasang. Metadata per-provider hanya diisi dari kemampuan yang *betul-betul* diuji.

### 5.4 Aturan menggabungkan evidence

- Semantic facts terverifikasi dapat lebih spesifik dari syntax candidates, tetapi **tidak boleh diam-diam menghapus** conflicting evidence.
- Simpan provenance: `syntax`, `LSP`, `SCIP`, `manual-verified`, identitas provider, versi, dan snapshot source.
- Dynamic/reflection, generated code, native bridges, template expansion, macro dan missing build configuration harus bisa dilabeli `unresolved`.
- Tidak menafsirkan **tidak ditemukan hubungan** sebagai **pasti tidak memiliki hubungan** bila provider tidak lengkap.
- Hubungan antarlanguage hanya dibuat ketika ada bukti eksplisit (misalnya binding/FFI mapping), bukan dengan menyamakan nama fungsi.
- Query `impact` harus mengembalikan `UNKNOWN` bila area analisis tidak lengkap; jangan memberi `LOW/SAFE` tanpa evidence yang layak.

---

## 6. Normalized graph dan data contract

### 6.1 Graph untuk kode

**Node:** `Repository`, `Revision`, `File`, `Module`, `Symbol`, `Package`, `ExternalSymbol`.  
**Edge:** `DECLARES`, `DEFINES`, `REFERENCES`, `CALLS`, `IMPORTS`, `IMPLEMENTS`, `EXTENDS`, `DEPENDS_ON`, `BINDS_TO` (hanya bila terbukti), dan `POTENTIAL_CALL` untuk heuristik.

Setiap edge memiliki:

- `repo_id`, `source_revision`, `provider_name`, `provider_version`;
- `from_id`, `to_id` atau unresolved target;
- `relation_type`, `precision`, `completeness` (`exact`, `lower_bound`, `unresolved`);
- `source_path`, byte offset/range, line/column untuk presentasi;
- `evidence_id`, `source_content_digest`, `last_verified_at`;
- `failure_reason` atau `conflict_reason` jika relevan.

**Identitas simbol:** gunakan repository namespace + language + provider identity bila tersedia; simpan alias dan location secara terpisah. Jangan menjadikan `path:line` atau `function_name` sebagai identitas permanen, sebab refactor dan rename mengubah keduanya. Jangan menganggap symbol ID SCIP stabil melintasi semua indexer, versi, atau build tanpa kontrak sumbernya.

### 6.2 Graph untuk knowledge

**Node:** `Decision`, `ADR`, `Requirement`, `Constraint`, `Experiment`, `Benchmark`, `Incident`, `Discovery`, `Component`, `Release`, `Task`, `ExternalDocument`.  
**Edge:** `AFFECTS`, `JUSTIFIES`, `VALIDATES`, `CONTRADICTS`, `SUPERSEDES`, `CONSTRAINS`, `CAUSED_BY`, `REFERENCES`, `RELATES_TO`.

Setiap fakta memiliki:

- stable `knowledge_id`, jenis, title, body dan metadata;
- provenance sumber asli (`path`, URL bila diimpor, hash, author bila diketahui);
- version/revision, status `active|superseded|retracted|draft`;
- `observed_at`, `recorded_at`, dan klaim berlaku sejak kapan bila diketahui;
- evidence links (dapat menunjuk file+range+Git state);
- `human_confirmed` / `machine_suggested` dan konflik yang masih terbuka.

**Tidak ada fakta otomatis hanya karena LLM menebaknya.** Ekstraksi AI boleh membuat **candidate assertions** dalam staging; promosi menjadi fakta terkonfirmasi membutuhkan evidence dan aturan/human approval.

### 6.3 Cross-link kode + knowledge

Hubungkan melalui edge berdasar evidence, misalnya:

```text
Function `merge_depth_maps`
   <-- AFFECTS -- Decision "fusion behavior"
   <-- VALIDATES -- Benchmark "depth quality regression"
   <-- CONSTRAINS -- Requirement "RAM <= user-defined limit"
```

Jika nama fungsi berganti, gunakan source history/verified symbol mapping; jangan melekatkan ADR lama ke simbol baru hanya dengan fuzzy matching.

### 6.4 Desain DB minimum

```text
repositories, repository_snapshots, files, file_digests,
symbols, symbol_occurrences, code_edges, provider_runs,
index_revisions, index_health,
knowledge_records, knowledge_revisions, knowledge_edges,
evidence, source_documents, document_revisions,
search_fts, schema_migrations
```

- Pisahkan namespace `repo_id` dan `workspace_id`; content-addressing boleh berbagi blob tanpa membocorkan isi lintas project.
- FTS5 untuk lexical search dan ranking dasar; graph traversals dapat memakai CTE recursive yang dibatasi depth/node count.
- Gunakan transaksi SQLite, staging tables/index file candidate, integrity checks, lalu atomic promotion. Jangan mengandalkan satu indeks global tanpa snapshot boundary.
- Bila benchmark menunjukkan graf besar jadi bottleneck, uji `petgraph` in-memory per subgraph dan **baru kemudian** pertimbangkan embedded graph DB. Menambah database bukan target produk.

---

## 7. Incremental indexing dan freshness (pembeda teknis utama)

### 7.1 Perubahan source yang harus dikenali

Tracked modification, untracked file, deletion, rename, branch switch, changed HEAD, changed build flags/config, dependency manifest, generated source, grammar/provider version, ignore policy, dan workspace path aliases.

### 7.2 Pipeline yang diusulkan

1. **Discover:** inventory file dengan ignore rules, Git state, safe path resolution, manifest workspace dan provider versions.
2. **Snapshot:** source revision + tracked/untracked content digest + relevant config fingerprint.
3. **Diff:** cari file yang benar-benar berubah, dihapus, ditambah, berubah status/konfigurasinya.
4. **Dependency invalidation:** tentukan batas safe invalidation; import/header/build config dapat memengaruhi banyak file.
5. **Extract:** parse hanya unit affected; provider semantic dapat memilih rebuild lebih luas jika diperlukan.
6. **Normalize:** bangun node/edge sementara beserta completeness/provenance; tandai unresolved edges.
7. **Validate:** integrity, source digest, dependency impact, referential consistency, conflict, dan cross-provider consistency.
8. **Promote:** commit snapshot baru secara atomik; hasil query melihat satu revision konsisten.
9. **Answer:** tampilkan data mutakhir dan peringatan cakupan yang belum terverifikasi.

### 7.3 Invariant wajib

- Tidak pernah menyajikan indeks stale sebagai `current`.
- Index commit atomic atau mempertahankan snapshot lama yang **secara eksplisit stale**, bukan memadukan catatan dua revision tanpa label.
- Rename/delete menghapus edge dan definition lama di revision baru.
- Perubahan interface/header/global symbol memicu pemeriksaan konsumen yang relevan.
- Provider gagal/timeout tetap menghasilkan output terbatas yang menyatakan keterbatasan.
- Perubahan ignore/filter/security policy membatalkan cache yang terkait; jangan mewarisi file-admission lama secara buta.
- Cache tidak valid harus bisa dihapus dan dibangun ulang dari source yang sama.
- Query read-only tidak menjalankan kode dari repository.
- Concurrent indexing + query diuji dengan snapshot isolation, cancellation, crash/restart recovery.

### 7.4 Tingkat incremental yang realistis

- **File-level syntax incremental** lebih dulu; Tree-sitter incremental AST saja **tidak menyelesaikan** dependency invalidation lintas file.
- **Dependency-aware selective semantic refresh** selanjutnya, hanya untuk provider/bahasa yang dapat dibuktikan aman.
- **Full provider regeneration** tetap boleh sebagai fallback resmi apabila presisi tidak bisa dipertahankan secara incremental.
- Bandingkan cost total (index time, RAM, correct edges, I/O) dengan full rebuild; jangan membuat arsitektur incremental kompleks bila lebih mahal tanpa manfaat yang terbukti.

---

## 8. Knowledge engine tanpa Obsidian

### 8.1 Storage contract

- **Canonical data proyek baru:** SQLite structured knowledge store dengan revision journal append-only secara logis, migration, backups, dan checksums.
- **Import/export:** Markdown + front matter yang terdokumentasi, JSONL untuk data graph dan revisions, serta ekspor yang deterministik untuk Git tracking.
- **Source evidence:** dokumen di repo, issue URL, commit reference, log, dan file lokal **tetap sumber bukti** jika record diekstrak darinya.
- **User edits:** record create/update/supersede/retract melalui CLI/API, dengan audit trail. Hindari update langsung database melalui skrip yang melewati aturan validasi.
- **Graph bukan file-link viewer:** edge bertipe dan divalidasi, bukan sekadar hubungan hyperlink Markdown.
- **Obsidian plugin:** boleh nanti jika komunitas menginginkan, tetapi tidak ada dependency Obsidian sama sekali.

### 8.2 Retrieval knowledge

1. Filter workspace/repo + permission scope.
2. Lexical search FTS5 dan filter tipe/status/waktu.
3. Graph neighborhood expansion yang relevan dan dibatasi.
4. Scoring by source quality, freshness, applicability, recency hanya bila relevan, dan unresolved/conflict flags.
5. Kembalikan ringkasan + sumber + alasan hubungan; full original document hanya on-demand.
6. Jika pertanyaan mengenai **apa yang kode lakukan sekarang**, source/Git current mengalahkan knowledge historis.
7. Jika pertanyaan mengenai **mengapa keputusan dibuat**, tampilkan decision+evidence+revision, bukan menebak dari kode.

### 8.3 Contoh kontrak record (usulan)

```json
{
  "id": "decision:local:0001",
  "kind": "Decision",
  "title": "Use a native embedded database",
  "status": "active",
  "body": "Keep local operation without a database server.",
  "revision": 1,
  "provenance": [{"type": "document", "uri": "docs/architecture.md", "digest": "sha256:<actual>"}],
  "edges": [{"type": "CONSTRAINS", "target": "component:storage"}],
  "recorded_at": "<RFC3339 timestamp>"
}
```

**Bukan data produksi:** ID/digest/timestamp adalah contoh format; isi record dan sumbernya harus benar-benar tersedia sebelum dibuat.

### 8.4 Sinkronisasi dan konflik

- Export deterministik, import idempotent, no silent overwrite.
- Optimistic locking/version precondition untuk edits.
- Reconcile conflict manual dan tampilkan provenance kedua sisi.
- Revisions tidak otomatis dihapus ketika source berubah; `superseded/retracted` memiliki history.
- Backup/restore/integrity check diuji dari database dan ekspor bersih.

---

## 9. Search, query, impact, trace, dan efisiensi token

### 9.1 Urutan retrieval

1. Query intent: symbol navigation, relationship, impact, trace, explanation, or historical reasoning.
2. Repository + revision + provider freshness check.
3. Candidate retrieval: exact symbol/qualified name, FTS5, filepath scope, document relevance.
4. Bounded graph traversal dengan hop limit dan page size.
5. Evidence ranking, conflict/dedup, status coverage.
6. Response compact: summary, path/range, edge/evidence, completeness, cursor/continuation.
7. Source excerpt atau knowledge original hanya saat diminta atau benar-benar relevan.

### 9.2 Contract output

```json
{
  "query": "callers of Example.process",
  "repository": "example-repo",
  "revision": "<actual-git-or-source-fingerprint>",
  "freshness": "current|stale|unknown",
  "completeness": "exact|lower_bound|unresolved",
  "results": [{
    "symbol": "Example.calling_method",
    "relation": "CALLS",
    "path": "src/example.rs",
    "range": {"start_line": 42, "end_line": 42},
    "provider": "syntax|lsp|scip",
    "evidence": "<short-verifiable-reference>"
  }],
  "unresolved": [],
  "truncated": false
}
```

### 9.3 Reliability pada `impact` dan `trace`

- **Direct impact** = inbound references/callers/import dependents yang dibuktikan.
- **Transitive impact** = traversal bounded, setiap edge punya provenance.
- **Risk ranking** = deterministic rules + explicit unknowns; bukan sekadar skor LLM.
- **Trace** = static plausible path, **bukan jaminan runtime execution**. Dynamic dispatch, reflection, FFI dan dependency injection wajib dilabeli.
- Pada area kosong karena unsupported, kembalikan `UNKNOWN`, bukan `SAFE`.

### 9.4 Token budget

- Default tool result harus singkat; `summary`, `limit`, `depth`, `include`, `cursor` disediakan.
- Ukur **total token semua tool calls + final context** sampai tugas coding diselesaikan, bukan hanya panjang satu response.
- Sediakan `--format json` stabil dan `--format compact` untuk manusia/agent.
- Bandingkan baseline manual lexical/grep dengan sistem baru menggunakan tugas identik; jangan klaim hemat sebelum lulus.

---

## 10. CLI & MCP public interface

### 10.1 CLI native (usulan kontrak, bukan command yang sudah ada)

```powershell
ocik init --repo D:\Projects\sample
ocik repo list
ocik repo status --repo sample
ocik index --repo sample --mode syntax
ocik provider doctor --repo sample
ocik index --repo sample --mode semantic --provider clangd
ocik search "process" --repo sample --format json
ocik context "Example.process" --repo sample --include definition,references,callers,callees
ocik impact "Example.process" --repo sample --depth 2 --format json
ocik trace "main" --repo sample --to "Example.process"
ocik knowledge add --file decision.md --repo sample
ocik knowledge search "why use sqlite" --repo sample
ocik knowledge export --format jsonl --output knowledge.jsonl
ocik mcp serve --transport stdio
ocik bench --corpus tests/corpus-v1.json
```

### 10.2 MCP tool surface MVP

- `repo_list`, `repo_status` — registry and freshness.
- `code_search` — source/symbol discovery.
- `code_context` — definitions, references, callers, callees, imports.
- `code_impact` — evidence-backed blast radius.
- `code_trace` — bounded candidate execution/dependency path.
- `knowledge_search` — verified knowledge retrieval.
- `knowledge_get` — get full record/evidence.
- `knowledge_related` — bounded graph neighborhood.
- `system_doctor` — capabilities/provider health and missing deps.

Optional post-MVP: `code_changes`, `knowledge_propose`, `code_explain`, `code_compare`, cross-repo links, search explainability.

**Protocol invariant:** MCP stdio output di `stdout` hanya JSON-RPC messages; log, progress, warning di `stderr`. CLI dan MCP memanggil shared application API yang sama. Validasi input, request limit, cancellation, timeout, and versioned schemas. MCP code/knowledge reading **read-only by default**; write tools wajib explicit opt-in.

---

## 11. Keselamatan, privasi, dan supply chain

### 11.1 Local-first

- Tidak ada upload source, indexing telemetry, provider cloud, atau panggilan model external secara default.
- Network disabled-by-default untuk core; provider eksternal hanya mengikuti kebijakan eksplisit.
- Remote MCP HTTP **bukan MVP**; jika nanti ditambah, butuh auth, bind restrictions, origin checks, dan rate limits.

### 11.2 Path & repository isolation

- Izinkan hanya root repo yang terdaftar; validasi symlink/junction/realpath, file identity, path traversal.
- Respect `.gitignore` yang tepat untuk use case, ditambah `.ocikignore` sebagai konfigurasi eksplisit; default skip `.git`, binaries, dependencies, large artifacts; alasan pengecualian harus bisa diaudit.
- Jangan membuka file besar atau secrets untuk hasil jawaban tanpa policy; masking/disclosure policy diuji.
- Tidak menjalankan source project hanya untuk parsing. Language servers/indexers bisa memicu proses/subprocess atau project hooks, sehingga jalur provider eksternal **harus opt-in dan dikonfigurasi secara aman**, bukan diasumsikan tidak mengeksekusi kode.
- Workspace A tidak boleh menghasilkan fakta/source dari Workspace B walau nama simbol sama.

### 11.3 Reliabilitas & distribusi

- Database integrity checks; atomic index promotion; schema migration + rollback plan; crash recovery.
- Version pinning, `Cargo.lock`, provenance/SBOM dan dependency/license scan setiap rilis.
- Self-hosted Windows + Linux/macOS CI; signed/checksummed release artifacts bila infrastruktur tersedia.
- Tes supply chain untuk grammar dan indeks provider, malformed SCIP, zip/path bombs, path traversal, long filenames, Unicode, CRLF/LF, case folding di Windows.

---

## 12. Lisensi dan model bisnis open-source

### 12.1 Lisensi proyek: Apache-2.0

**Mengapa:** OSI-approved, mengizinkan penggunaan komersial/fork/distribusi, dan memberikan patent grant eksplisit beserta notice obligations. Penggunaan komersial tidak memerlukan pembayaran kepada pengelola proyek **karena proyeknya menggunakan Apache-2.0**, sepanjang pengguna mematuhi lisensi dan semua dependensi terkait.

**Jangan:** memilih PolyForm Noncommercial, BSL, SSPL, atau lisensi pembatas penggunaan komersial jika produk ingin dipromosikan sebagai **permissive open-source**. Lisensi ini tidak setara dari sisi kepatuhan open-source dan kebebasan adopsi bisnis.

### 12.2 Tabel pemeriksaan dependency (per 14 Oktober 2026)

| Komponen | Lisensi yang ditemukan di sumber primer | Aksi sebelum distribusi |
|---|---|---|
| Tree-sitter core | MIT | Sertakan notices; cek setiap grammar terpisah |
| SCIP specification/tooling repo | Apache-2.0 | Catat versi/proto origin |
| `scip-typescript` | Apache-2.0 | Periksa npm transitive deps & izin redistribusi |
| `scip-dotnet` | Apache-2.0 | Periksa NOTICE/transitive deps sebelum bundle |
| clangd/LLVM | Apache-2.0 WITH LLVM-exception | Pertahankan notice dan verifikasi distribusi LLVM/toolchain |
| rust-analyzer | MIT OR Apache-2.0 | Catat opsi lisensi paket rilis |
| `rmcp` SDK | Repo memiliki transisi MIT→Apache-2.0; kontribusi lama dapat tetap MIT | Simpan full LICENSE yang berlaku pada versi dipin |
| `rusqlite` | MIT; SQLite bundled public domain | Audit fitur tambahan dan bundled libs |
| `gix` (gitoxide) | MIT OR Apache-2.0 | Audit versi/dependency flags |
| `petgraph` | MIT OR Apache-2.0 | Tidak perlu sebagai storage, hanya opsional algoritma |
| LadybugDB (tidak wajib) | MIT pada repo saat diperiksa | Evaluasi ulang bila suatu hari jadi backend |
| GitNexus | **PolyForm Noncommercial 1.0.0** | **Jangan copy/reuse source ke produk komersial tanpa izin yang sesuai** |
| Bahasa/grammar/indexer lain | **Belum otomatis terverifikasi** | Audit **tiap paket dan versi** sebelum bundling |

**Peringatan:** perizinan kombinasi library dan binary yang didistribusikan bergantung pada **seluruh dependency graph**, bukan label lisensi komponen utama saja. Tabel adalah penelitian awal, bukan legal opinion atau SBOM final.

### 12.3 Tata kelola kontribusi

- Sediakan `LICENSE` Apache-2.0 lengkap, `NOTICE` jika diperlukan, `CONTRIBUTING.md`, `SECURITY.md`, dan SPDX headers untuk file yang cocok.
- Pakai **DCO sign-off** sebagai kebijakan kontribusi awal; pastikan kontributor mempunyai hak atas code dan tidak memasukkan kode noncommercial/restricted.
- Pengelola tidak membutuhkan CLA eksklusif untuk proyek Apache-2.0 standar. Jika kelak dual-license secara komersial membutuhkan hak tambahan, bahas secara transparan sebelum meminta izin contributor.
- Nama/logo merek dagang diatur terpisah; Apache-2.0 tidak otomatis memberi hak merek.
- Model bisnis yang **tetap menjaga core Apache-2.0**: managed hosting opsional, support, enterprise integration, SLA, training, dan custom development. Jangan menjanjikan fitur utama hanya bagi pembayar jika tujuan komunitas adalah core yang terbuka; kebijakan edisi disepakati transparan.

### 12.4 Independensi dari GitNexus dan TK

- Tidak mengunduh atau menyalin kode GitNexus untuk membangun proyek ini.
- Jangan menggunakan hasil kode internal TK, schema, fixture, atau knowledge database sebagai seed proyek baru.
- Pengembangan fitur berbasis kebutuhan original, spesifikasi publik yang legal diakses, standar terbuka, dan test corpus yang dibuat sendiri/berlisensi cocok.
- Jika ingin melakukan benchmark langsung dengan instalasi GitNexus, **pastikan izin penggunaan itu sesuai lisensinya** untuk konteks aktivitas tersebut; alternatif aman adalah source-truth dan benchmark baseline non-restricted yang dibuat sendiri serta perbandingan fitur dari dokumentasi publik.
- Dokumentasikan asal bahan, source permission, dan signed contributor provenance.

---

## 13. Matriks kompetisi terhadap GitNexus

| Dimensi | Baseline kompetitor yang terlihat publik | Sasaran proyek baru | Bukti yang diwajibkan |
|---|---|---|---|
| Lisensi | GitNexus PolyForm Noncommercial | Apache-2.0, komersial tanpa lisensi tambahan atas core | LICENSE, SBOM, compliance scan |
| Deployment | GitNexus lokal CLI/MCP | Single native Rust core binary, offline | Windows/Linux/macOS smoke |
| Syntax multi-language | Ada parser/code graph | Structural baseline multi-bahasa dengan coverage/limits eksplisit | Parser corpus per bahasa |
| Semantic relationships | Code graph/call relationships | LSP/SCIP dengan evidence tingkat presisi | Source-verified references/calls |
| Impact & trace | Tersedia | Bounded, source-backed, low false-safety | Golden change-impact cases |
| Freshness | Lokal/indexing | No silent stale, file-level delta + safe invalidation | Rename/delete/branch test |
| Search relevance | Hybrid code search tersedia | Compact, deterministic lexical+graph baseline, semantic opt-in | Task retrieval benchmark |
| Persistent engineering knowledge | Bukan diferensiasi utama berbasis knowledge typed graph | Typed decision/constraint/benchmark graph | History/provenance evaluation |
| Resource use | Perlu diukur pada environment sama | Bounded and published memory/disk/index metrics | Matched hardware & corpus |
| Business adoption | Penggunaan komersial tunduk lisensi kompetitor | Penuh sesuai Apache-2.0 | Release legal checklist |

**Bukan klaim saat ini:** hanya kolom sasaran, tidak ada benchmarking implementasi proyek baru yang sudah dijalankan. Jangan tulis “faster”, “more accurate” atau “better than GitNexus” di README sebelum syarat bukti tercapai.

---

## 14. Goals dan definisi keberhasilan

### 14.1 North-star outcome

**G-NS:** Developer/AI agent dapat memahami **apa yang kode lakukan, apa dampak perubahan, dan mengapa keputusan terdahulu dibuat**, secara lokal dan legal untuk bisnis, dengan bukti sumber, biaya konteks terukur, dan tanpa menerima fakta stale sebagai current.

### 14.2 Target yang mengikat (release gates)

| ID | Goal | Ukuran kelulusan |
|---|---|---|
| G-LIC | Open-source & business-safe | Apache-2.0 core, SBOM, izin tiap dependency yang dibundel, 0 kode restricted tidak berizin |
| G-NAT | Native cross-platform | Same Rust core binary berfungsi di Windows/Linux/macOS via CI; tidak perlu Python/Node untuk syntax/knowledge baseline |
| G-CLI | Stable CLI | Versioned command/JSON contract, deterministik untuk fixtures, meaningful exit codes |
| G-MCP | MCP ready | stdio smoke pada minimal 2 klien uji kompatibel, valid JSON-RPC, no stdout logs |
| G-SYN | Syntax intelligence | Python/TS/Rust/C++ corpus source-verified; unsupported konstruksi diberi label |
| G-SEM | Semantic intelligence | Minimal 2 provider keluarga bahasa berbeda lulus source-grounded definitions/references (salah satunya C/C++ atau TS) |
| G-GRF | Relationship graph | Callers/callees/imports/impact/trace edge evidence dan coverage semantics; 0 false `SAFE` dalam mandatory safety corpus |
| G-FSH | Freshness | 0 silent-stale pada corpus dirty/rename/delete/branch/config/provider-failure yang didefinisikan |
| G-INC | Incremental | Correctness parity vs full rebuild dan material improvement untuk 1-file modifications dibanding full rebuild dalam corpus yang sama |
| G-KNW | Knowledge graph | CRUD+revision+typed edges+provenance+conflict+export/import+backup/restore lulus; tanpa Obsidian |
| G-TOK | Agent usefulness | Token/task completion dan evidence relevance minimal tidak lebih buruk dari manual baseline pada frozen tasks |
| G-SEC | Privacy/security | No source uploads by default; root isolation, symlink/junction, adversarial provider inputs, crash recovery tests green |
| G-PAR | Competitive validity | Scope terbatas; unggul pada ≥3 dari 5 dimensi terukur *yang ditetapkan sebelum eksperimen*, tanpa regression kritis correctness |

**Penjelasan G-PAR:** lima dimensi yang dibekukan ialah **(1) source-grounded correctness, (2) change freshness, (3) changed-file refresh cost, (4) agent token/task success, (5) maintainability/deployment burden**. Jika benchmark GitNexus langsung tidak diizinkan oleh lisensi, gunakan pembanding independen yang sah dan nyatakan **tidak ada head-to-head claim**. Keunggulan Apache-2.0 dievaluasi terpisah dari performa.

### 14.3 Initial engineering SLO (proposal, bukan angka benchmark aktual)

Untuk fixtures kecil/menengah yang disepakati, target pertama:

- Warm indexed symbol/relationship queries: **p95 ≤ 500 ms**, **tidak termasuk cold language-server initialization**.
- Syntax-only changed-file refresh: **p95 ≤ 2 detik** untuk perubahan kecil pada repo pilot sekitar 2.000 file di hardware benchmark yang dibekukan; harus tetap correctness parity.
- End-to-end MCP stdio overhead terhadap shared core: ukur dan targetkan **p95 ≤ 200 ms overhead** untuk request lokal non-indexing (tidak mengklaim total response).
- Memory/binary/disk: publikasikan idle/peak RSS dan index size; **belum ada batas keras yang defensible** sebelum baseline pengukuran.
- Semantic provider-specific cold index harus dilaporkan terpisah dari syntax core, tanpa menyembunyikan biaya startup dan toolchain.

Ambang di atas adalah **sasaran awal yang dapat direvisi secara terdokumentasi setelah baseline**, bukan janji performa. Jika target gagal, laporkan; jangan menurunkan validasi keamanan/akurasi untuk sekadar memenuhi angka.

### 14.4 Golden evaluation corpus

- Minimal empat bahasa pilot: Python, TypeScript, Rust, C++.
- Fixtures self-authored/berlisensi jelas: direct calls, class methods, import aliases, ambiguity, recursion, cycles, symbols with same names, missing imports, unsupported syntax.
- Edge cases: dynamic dispatch, reflection, macro, template, header changes, ambiguous overload, native/FFI boundary, workspace/branch changes, dirty/untracked, rename/delete, CRLF, symlink/junction.
- Pertanyaan nyata: `who calls`, `what changes if`, `why decided`, `trace to`, `what is stale`, `what is unsupported`.
- Gunakan dua cohort: **synthetic ground-truth fixtures** + **real open-source repos berlisensi sesuai**. Pisahkan keduanya di laporan; hindari overfitting ke corpus tunggal.
- Manual evidence review/annotation oleh maintainer untuk sampel kritis; missing relation dinilai beda dari false relation.

### 14.5 Metrik wajib

- Semantic precision, recall/coverage, false positives/false negatives, unsupported cases.
- Zero-false-safe **pada corpus safety yang dibekukan**; tidak menjanjikan zero false-safe universal.
- Full/cold indexing; warm query p50/p95/p99; changed-file refresh p50/p95; provider process startup.
- Peak RSS, idle RSS, index disk bytes, count files parsed/reparsed/written, cache hit/miss.
- Tool call count, total agent tokens per completed task, correctness of final task answer, number of source files read.
- Windows/Linux/macOS pass/fail; license scan results; SBOM; security boundary results.

---

## 15. Roadmap berurutan (setiap tahap ada STOP/GO gate)

> **Tidak mengeksekusi pembangunan tahap berikutnya sebelum milestone sekarang mempunyai bukti PASS.** Roadmap adalah proposal proyek baru; **bukan instruksi untuk mengubah TK internal**.

### M0 — Open-source project bootstrap & legal baseline

**Goal:** repositori bersih, lisensi dan struktur awal sah serta terisolasi.

**Deliverables:** Rust workspace kosong/compile, README visi, Apache-2.0 LICENSE, code ownership, SPDX/notice policy, DCO, CI basic, security policy, manifest dependency/licenses, public demo fixtures.

**PASS:** `cargo fmt/check/test` green; batas independensi tercatat; audit awal seluruh dependency/grammar; nama/merek belum diklaim tersedia.

**STOP:** ditemukan source restricted/tidak jelas hak penggunaan, atau code terbawa dari TK/GitNexus.

### M1 — Native CLI + repository registry

**Goal:** satu binary mampu mendaftarkan repo, mengetahui worktree, dan mengeluarkan JSON machine-readable.

**Deliverables:** `init`, `repo list`, `repo status`, `doctor`, Git fingerprint, ignore policy, path security, Windows/Linux CI.

**PASS:** dirty/untracked/branch + symlink/junction/permission/corrupt repo cases lulus; repo isolation terbukti.

### M2 — Tree-sitter syntax baseline

**Goal:** parse source dan hasilkan definisi/imports/local call candidates.

**Deliverables:** grammar Python/TS/Rust/C++, capture queries pinned, source spans, syntax graph, syntax search.

**PASS:** fixtures tiap bahasa source-verified, deterministic reindex, malformed syntax recoverable; `unsupported` tidak disamarkan.

### M3 — Durable SQLite code graph & freshness

**Goal:** single-snapshot indexing yang aman dan dapat dibuat ulang.

**Deliverables:** schema/migration, SQLite/FTS5, content fingerprints, atomic replacement/staging, stale diagnostics, exact source evidence.

**PASS:** rename/delete/config/HEAD/dirty/untracked, crash/restart/corrupt cache tests; query tidak silent-stale.

### M4 — Safe incremental syntax refresh

**Goal:** perubahan kecil tidak lagi selalu memerlukan full parse.

**Deliverables:** per-file extraction cache, invalidation closure minimal aman, fallback rebuild, counters, benchmark harness.

**PASS:** parity graph dengan full rebuild untuk test corpus; 1-file delta materially cheaper; tidak melemahkan security screening.

### M5 — Semantic provider protocol (LSP + SCIP)

**Goal:** semantic evidence masuk ke graph tanpa menghilangkan provenance.

**Deliverables:** LSP subprocess adapter, generic SCIP streaming decoder/importer, provider capability registry, conflict diagnostics, timeout/cancellation, safe opt-in.

**PASS:** minimal dua provider lintas bahasa, definition/reference verifiable, fallback syntax bila provider absent/fail, no dependency required untuk core-only.

### M6 — Relationship, trace & impact product capability

**Goal:** fitur yang bernilai untuk refactor harian.

**Deliverables:** callers/callees, dependency traversal, `impact`, `trace`, `UNKNOWN` completeness, evidence chain, bounded query.

**PASS:** golden corpus correctness + false-safe gate; all output points to source; clear unsupported boundary.

### M7 — Native knowledge graph tanpa Obsidian

**Goal:** permanent, revisioned engineering memory yang bisa dibaca agent.

**Deliverables:** knowledge records/typed edges, evidence, revisions, conflict/retraction, lexical search, export/import MD+JSONL, backup/restore.

**PASS:** lossless round-trip pada supported fields, idempotent import, provenance, no external app dependency, accurate graph links.

### M8 — Unified CLI/MCP & agent efficiency

**Goal:** code+knowledge dalam shared API yang praktis untuk manusia/AI.

**Deliverables:** `rmcp` stdio server, contract schemas, compact responses, pagination/budgets, host integrations, token/task benchmark.

**PASS:** minimal 2 agent clients interoperability; JSON schema validation; source correctness; task-token benchmark setara atau lebih baik dari manual baseline.

### M9 — Pilot multi-repo & release candidate

**Goal:** project-grade pada beberapa repo open-source nyata, Windows/Linux/macOS.

**Deliverables:** reproducible benchmark, security/compatibility tests, packaging native releases, SBOM/notices, documented language support.

**PASS:** semua G-LIC..G-SEC; known limitations published; no blocker high severity. Rilis `v0.1.0` hanya bila MVP benar-benar dapat digunakan.

### M10 — Competitive evidence & public launch

**Goal:** bandingkan secara sah dan umumkan klaim terbatas yang terbukti.

**Deliverables:** public benchmarking methodology/results, reproducible corpus, legal evidence of competitor comparison if performed, OSS community onboarding docs.

**PASS:** G-PAR sesuai pembanding yang sah; jika tidak tercapai, rilis tetap boleh dilakukan tanpa klaim melampaui GitNexus. Jangan menahan manfaat open-source dasar hanya demi narasi kompetitor.

### Post-MVP options (tidak otomatis dibangun)

Go/Java/C# support, CUDA-specific semantic provider, cross-repo semantic links, local embeddings, optional graph UI/TUI, package dependency vulnerability awareness, multi-user/team collaboration, hosted instance, change-impact review integration. Prioritas selalu dari bug/permintaan pengguna dan benchmark.

---

## 16. First implementation backlog (urutan issue yang siap dibuat)

| Issue | Ringkasan | Owner modul | Dependency | Bukti akhir |
|---|---|---|---|---|
| #1 | OSS license + NOTICE + DCO + CI bootstrap | governance | - | Compliance check green |
| #2 | CLI skeleton + typed JSON results | cli/core | #1 | CLI snapshot tests |
| #3 | Multi-repo registry + filesystem safety | repo | #2 | Symlink, worktree, repo isolation tests |
| #4 | Tree-sitter captures 4 bahasa | parser | #3 | Golden symbol/location tests |
| #5 | Normalized graph data contract | graph | #4 | Evidence/completeness tests |
| #6 | SQLite schema + staging + FTS | storage | #5 | Atomicity/corruption tests |
| #7 | Git/source freshness+rebuild | repo/storage | #6 | Dirty/rename/delete cases |
| #8 | File-local cache & selective syntax refresh | parser/storage | #7 | Parity+profiling benchmark |
| #9 | LSP adapter & provider policy | providers | #8 | Definitions/references fixture |
| #10 | SCIP decoder & cross-provider conflict | providers | #9 | Valid/malformed SCIP + provenance |
| #11 | Callers/callees/impact/trace | graph/search | #10 | No-false-safe corpus |
| #12 | Knowledge canonical schema/revisions | knowledge/storage | #6 | CRUD+audit tests |
| #13 | Knowledge graph queries/import/export | knowledge/search | #12 | Round-trip tests |
| #14 | MCP stdio facade | mcp | #11,#13 | 2 clients smoke |
| #15 | Compact task retrieval/benchmark | evaluation | #14 | Token/task outcomes |
| #16 | Multi-OS release and license SBOM | release | #15 | Signed/checksummed release + notices |

**Tidak perlu membuat semua 16 issue sekaligus** jika pengelolaan satu per satu lebih efektif. Paling awal cukup milestone M0–M2, sisanya sebagai backlog terencana.

---

## 17. Risiko utama dan mitigasi

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Kualitas semantik berbeda antar bahasa | False links/refs | Capability matrix, precision labels, gold corpus per bahasa |
| C++/CUDA compilation database sulit | Relasi tidak lengkap | clangd opt-in, platform configs, explicit unresolved |
| SCIP indexer/grammar lisensi/transitive berubah | Risiko distribusi bisnis | Version pin + automated license audit + no blind bundling |
| LSP provider menjalankan project tooling | Security/trust risk | Explicit consent, process policy, sandbox/execution boundaries |
| Delta indexing merusak cross-file edges | False SAFE | Full rebuild parity, dependency invalidation tests |
| Query graph terlalu mahal pada repos besar | Latency/RAM | Indexed adjacency, depth limits, bounded traversal, profiling |
| Semantic toolchain mempersulit native installation | Kegagalan onboarding | Core-only default + provider doctor/instructions |
| Knowledge menjadi kumpulan ringkasan tanpa bukti | Hallucinated history | Provenance, revision journal, human approval for proposals |
| Token reduction dikejar dengan memotong evidence | Hasil misleading | Quality/recall gate mengikat sebelum efisiensi |
| Membandingkan/memanfaatkan GitNexus di luar izin lisensi | Legal exposure | No code reuse; legal review for direct competitor use; original corpus |
| Solo maintainer overengineering | Tidak pernah shipping | Modul monolith, STOP/GO milestones, defer cloud/UI/vector |

---

## 18. Perintah mulai proyek untuk coding agent (kontrak M0 saja)

> Ini adalah **brief** untuk memulai **repository baru yang kosong**, bukan perintah yang sudah dieksekusi.

```text
Buat repository BARU untuk project open-source native code intelligence +
engineering knowledge graph, sesuai file BLUEPRINT ini. Jangan mengubah,
membuka sebagai dependency, menyalin, atau memigrasikan TK internal.
Jangan mengimpor source GitNexus.

Kerjakan HANYA milestone M0:
- Rust workspace skeleton, CLI executable placeholder, no networking.
- LICENSE Apache-2.0 + NOTICE policy, CONTRIBUTING/DCO, SECURITY.
- Dependency and license inventory with exact versions and approved sources.
- README scope: native core, CLI/MCP future, optional semantic providers.
- CI lint/format/test on Windows, Linux, macOS as infrastructure allows.
- Self-authored small test fixtures; clean repository history.

Jangan bangun parser, graph, MCP, knowledge ingestion, embedding, atau UI
pada M0. Laporkan hasil tests, licensing risks, dan HEAD sebelum meminta
approval untuk M1. Semua claims harus evidence-backed.
```

---

## 19. Referensi primer dan bukti verifikasi

Sumber di bawah adalah dokumentasi/kode lisensi publik yang diperiksa pada **14 Oktober 2026**, bukan endorsement ataupun izin otomatis untuk menyalin seluruh kodenya. Lisensi harus dicek ulang di versi/commit yang benar-benar dipin ketika implementasi.

1. **GitNexus license (PolyForm Noncommercial):** https://github.com/abhigyanpatwari/GitNexus/blob/main/LICENSE  
   GitNexus README: https://github.com/abhigyanpatwari/GitNexus/blob/main/gitnexus/README.md
2. **Open Source Initiative Apache-2.0:** https://opensource.org/license/Apache-2.0
3. **Tree-sitter (MIT), native parser dan Rust bindings:** https://github.com/tree-sitter/tree-sitter
4. **SCIP spec/protobuf (Apache-2.0):** https://github.com/scip-code/scip/ dan https://github.com/scip-code/scip/blob/main/scip.proto
5. **Official Rust MCP SDK (`rmcp`), repo LICENSE termasuk peralihan MIT/Apache:** https://github.com/modelcontextprotocol/rust-sdk dan https://github.com/modelcontextprotocol/rust-sdk/blob/main/LICENSE
6. **SQLite FTS5:** https://www.sqlite.org/fts5.html ; **recursive CTE:** https://www.sqlite.org/lang_with.html
7. **`rusqlite` MIT + bundled SQLite public domain:** https://github.com/rusqlite/rusqlite
8. **`gix`/Gitoxide MIT OR Apache-2.0:** https://github.com/GitoxideLabs/gitoxide
9. **`petgraph` MIT OR Apache-2.0:** https://github.com/petgraph/petgraph
10. **clangd indexing model:** https://clangd.llvm.org/design/indexing ; LLVM license overview: https://llvm.org/docs/DeveloperPolicy.html
11. **rust-analyzer LSP & license:** https://github.com/rust-lang/rust-analyzer
12. **SCIP TypeScript indexer, license Apache-2.0:** https://github.com/sourcegraph/scip-typescript
13. **SCIP Python indexer (provider prerequisites; license verify before bundling):** https://github.com/sourcegraph/scip-python
14. **SCIP .NET indexer, license Apache-2.0:** https://github.com/sourcegraph/scip-dotnet
15. **SCIP Clang indexer (Apache-2.0; platform/build integration separate):** https://github.com/sourcegraph/scip-clang
16. **LadybugDB MIT (evaluasi alternatif, bukan requirement):** https://github.com/LadybugDB/ladybug/blob/main/LICENSE
17. **Python LSP server (MIT, contoh provider optional):** https://github.com/python-lsp/python-lsp-server

### 19.1 Fakta vs rancangan

- **Fakta yang diverifikasi:** keberadaan framework/protokol, lisensi utama yang tercatat di sumber primer, pilihan SDK, fitur dasar yang didokumentasikan.
- **Rancangan dalam dokumen:** schema internal, nama CLI, kemampuan dan urutan MVP, target SLO, benchmark gates, dan kemungkinan mengungguli kompetitor.
- **Belum diverifikasi:** latency/CPU/RAM produk baru, akurasi cross-language, ketersediaan binary semantic indexer di seluruh OS, kepatuhan transitif dari lockfile rilis, dan hasil head-to-head benchmark.

---

## 20. Final recommendation

**Bangun proyek open-source ini sebagai produk mandiri Apache-2.0 dengan native Rust core, Tree-sitter syntax baseline, semantic provider adapter berbasis LSP/SCIP, SQLite-backed typed code/knowledge graph, dan dua pintu pemakaian — CLI serta MCP.**

Poin terpenting bukan mengejar seluruh fitur GitNexus secepat mungkin, melainkan menghasilkan **hubungan kode yang bisa diverifikasi, selalu jelas status freshness-nya, punya incremental refresh yang benar, knowledge graph yang dapat diaudit, dan respons efektif untuk manusia maupun agent**. Semua itu tetap bisa dicapai tanpa Obsidian, tanpa Node/Python runtime untuk core, dan tanpa database server besar.

**Langkah pertama:** M0 bootstrap/legal gate untuk repo **baru**. **TK internal tetap tidak disentuh.**
