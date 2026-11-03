# Rust alias: producer nyata, CLI–MCP, dan pilot agent

Tanggal 3 November 2026. Status: **hasil pada satu fixture**, bukan pengukuran kebenaran umum. Evaluasi memakai HEAD `72f3fa12bbe04e6cce3a2c1249f6d6744fd38144`, binary Riltra SHA-256 `215a1b4ac46e0d5063ae9d4be2e24ce3bcfead2b822dde249405609faf682a69`, dan Windows. Fixture dan home terpisah dari repository; snapshot tidak memiliki Git HEAD.

## Acuan dan batas

Fixture [caller precision](pilots/wiy-caller-precision-2026-10-31/source/src/lib.rs) digunakan kembali. Sebelum query untuk run ini, acuan berikut dicatat dari sumber:

| Baris | Acuan |
|---|---|
| 3 | `direct` memanggil `a::target()` |
| 4 | `alias` mereferensikan `a::target` sebagai initializer dan memanggilnya melalui `f()` |
| 5 | `reference` mereferensikan `a::target` tanpa memanggilnya |
| 6 | `other` memanggil `b::target()` |

Kedua `target` memiliki nama sama tetapi identitas berbeda. Oracle untuk run ini dibaca dari sumber dan dikunci sebelum query, tetapi tidak ditinjau oleh orang independen; fixture dan hasil lama sudah tersedia, sehingga ini bukan uji buta. Dua pemanggilan ke `a` dan satu ke `b` adalah tiga call expression dalam semesta yang didefinisikan ini. Dua initializer ke `a` adalah referensi, bukan call expression.

### Tinjauan susulan oleh sesi agent terpisah

Pada 3 November 2026, pemilik menyampaikan laporan tinjauan oleh sesi Claude Code terpisah yang diminta menyusun oracle dari `lib.rs` tanpa membaca hasil Riltra. Laporan tersebut menyebut SHA-256 source `26087be230ac7cf215a6009829aed84a380191737eff9d93c7dd8da2423115d1`, sesuai source yang diverifikasi ulang pada checkout ini. Tinjauannya menyatakan dua definisi berbeda, tiga pemanggilan (termasuk `f()` pada baris 4) dan dua referensi tanpa pemanggilan; identitas, baris dan perannya cocok dengan tabel oracle di atas. Tidak ada koreksi terhadap oracle atau hasil run sebelumnya.

Ini adalah **tinjauan susulan oleh sesi agent terpisah**, bukan tinjauan yang dilakukan sebelum run semantic. Log lokal sesi diperiksa secara read-only setelah pemilik menyampaikan laporan: tindakan tercatat menunjukkan pembacaan dan pencarian pada fixture serta hash yang sesuai, dan tidak menunjukkan pemanggilan Riltra atau pencarian internet. Sesi awal telah memuat subjek commit yang menyebut perbaikan alias, serta instruksi dan memori lain; sesi juga membaca dan menulis data task-observer di luar fixture. Log mentah tetap pada perangkat pemilik dan tidak disertakan dalam commit; SHA-256 saat diperiksa adalah `e03a6717a62b3c0bd1b1020d07586484b3101cac9d5217b8b0a1fece1be08dca`. Log membuktikan tindakan yang tercatat, tetapi tidak membuktikan pembatasan akses secara teknis atau seluruh tindakan tidak langsung skrip helper. Hasil ini memperkuat kesesuaian acuan pada fixture, tetapi tidak menjadikannya uji buta atau pemeriksaan manusia independen.

## Producer dan query langsung

`rust-analyzer 1.99.1` menghasilkan SCIP secara offline pada fixture tepercaya, dengan Cargo target di luar source dan `--exclude-vendored-libraries`. Producer dapat menjalankan build script atau procedural macro proyek; pengaturan offline tidak menjamin tidak ada eksekusi kode. Import melaporkan satu dokumen, 11 definisi, sembilan referensi, tanpa dokumen dilewati, dan `Unverified`. Producer log yang disimpan tidak memiliki baris `ERROR` atau `WARN`; itu bukan audit seluruh diagnostik atau bukti output lengkap.

Query `callers` pada kedua definisi menghasilkan:

| Anchor | Baris / isi sumber | Hasil |
|---|---|---|
| `a::target` | 3 `a::target()` | `CALLS`, target `a` |
| `a::target` | 4 `target` pada initializer | `REFERENCES`, target `a` |
| `a::target` | 4 `f()` | `CALLS`, target `a` |
| `a::target` | 5 `target` pada initializer | `REFERENCES`, target `a` |
| `b::target` | 6 `b::target()` | `CALLS`, target `b` |

Tiga lokasi panggilan sesuai oracle; tidak ada call tambahan dalam dua respons, tidak ada pertukaran target, dan kedua referensi tidak dipromosikan sebagai call. Slice byte setiap `caller_span` diperiksa pada sumber. Hasil fresh, tidak terpotong, `Unverified`, dan completeness `partial`; ini tidak menjamin hasil serupa untuk semua alias, tabrakan nama, atau bahasa Rust secara keseluruhan. `impact` untuk `a` mengembalikan `UNKNOWN` dan `partial`.

### Cara mengulangi query

Gunakan checkout pada pin di atas dan direktori scratch baru. Salin `docs/pilots/wiy-caller-precision-2026-10-31/source/` ke direktori source di scratch, buat lockfile dengan `cargo generate-lockfile --offline`, dan catat SHA-256 source, manifest, dan lockfile sebelum query. Bangun binary dari checkout terpin dengan `cargo build --locked`; simpan `RILTRA_HOME`, Cargo target, artifact SCIP, dan hasil di luar source. Pada run ini perintah berikut digunakan secara offline, dengan `<source>`, `<artifact>`, dan `<producer-log>` sebagai placeholder yang diganti operator:

```text
rust-analyzer --log-file <producer-log> scip <source> --output <artifact> --exclude-vendored-libraries
riltra init --repo <source> --name oracle
riltra index --repo oracle
riltra semantic import --repo oracle --scip <artifact>
riltra callers --repo oracle --path src/lib.rs --line 1 --col 20 --limit 100
riltra callers --repo oracle --path src/lib.rs --line 2 --col 20 --limit 100
riltra impact --repo oracle --path src/lib.rs --line 1 --col 20 --limit 100
```

Untuk MCP, inisialisasi server `riltra mcp`, kirim `notifications/initialized`, lalu panggil `riltra_code_callers` dengan `repo`, `path`, `line`, `col`, dan `limit` yang sama untuk kedua anchor; panggil `riltra_code_impact` dengan parameter anchor pertama. Bandingkan JSON CLI dengan `result.structuredContent`, bukan hanya teks ringkasan. Rekam semua output dan kegagalan. Pada salinan uji yang sama, tambahkan komentar ke `src/lib.rs`, kemudian ulangi `index status`, tiga query CLI, dan tiga query MCP untuk memeriksa freshness. Jangan menafsirkan relasi yang masih muncul pada snapshot stale sebagai fakta sumber terkini.

Untuk pilot agent, berikan salinan fixture identik kepada dua sesi terpisah. Minta masing-masing menginventarisasi call dan reference untuk kedua target, termasuk alias; hanya sesi kedua diminta memakai semantic `callers` Riltra sebelum memeriksa sumber. Simpan jawaban dan tool call kedua sesi, bandingkan dengan acuan di atas, dan catat bila agent menggunakan pencarian sumber sebagai fallback. Instruksi penggunaan tool berbeda, sehingga perbandingan tidak mengisolasi efek engine.

## CLI, MCP, dan sumber usang

Klien MCP lokal menginisialisasi server dan meminta `riltra_code_callers` pada kedua target dan `riltra_code_impact` pada `a`. `structuredContent` dari ketiga respons sama dengan JSON CLI yang bersesuaian pada snapshot fresh. Setelah komentar ditambahkan pada **salinan fixture uji saja**, status dan ketiga query melaporkan `stale`/`content-changed`; tiga respons MCP sekali lagi sama dengan CLI. Baris `CALLS` dan `REFERENCES` lama tetap ada dalam respons yang berlabel stale; hasil ini bukan bukti fakta tersebut masih mutakhir. `impact` stale tetap `UNKNOWN`.

Hash ketiga file source/manifest/lock sebelum dan sesudah rangkaian fresh identik. Perubahan untuk stale disengaja setelah pemeriksaan itu. Dua salinan untuk agent memiliki `src/lib.rs` SHA-256 `26087be230ac7cf215a6009829aed84a380191737eff9d93c7dd8da2423115d1` sesudah sesi.

## Pilot agent

Satu sesi baseline memakai pencarian sumber; satu sesi lain diminta menggunakan Riltra semantic dan kemudian memeriksa sumber. Keduanya menyebut tiga panggilan dan dua referensi yang benar. Baseline menggabungkan dua peran baris 4 dalam satu baris tabel; isinya menyatakan kedua peran. Agent Riltra menjalankan semantic status, semantic query, dan `callers` untuk kedua anchor, lalu membaca sumber. Agent tersebut menyebut `Unverified`, anchor `Heuristic`, dan completeness `partial`.

Ini menunjukkan kebenaran jawaban akhir pada dua sesi untuk fixture ini, bukan pembuktian kausal manfaat Riltra. Model efektif tidak ditetapkan atau dicatat; agent membaca skill di luar fixture; baseline mencatat timeout refresh katalog model tetapi menyelesaikan tugas. Hanya satu percobaan per kondisi, instruksi penggunaan tool berbeda, dan tidak ada randomisasi. Tidak ada klaim kecepatan, biaya atau produktivitas.

## Bukti dan batas lanjutan

Sebanyak 37 file bukti terpilih (query, respons MCP, logs, script dan source agent) disalin dan diverifikasi SHA-256 ke `docs/pilots/agent-kebenaran-2026-11-02/raw-local/rust-real-oracle-20261103/`. Direktori ini diabaikan Git, mengandung path lokal dan tidak tersedia dari commit. Log agent mentah SHA-256:

```text
145a880e7f912182737d8ceacbf53d44f3828a2cdb951f6a0652914cb2cb581a  agent-results/base-events.jsonl
37cd4123218e30960369fba540e118417f664d82623e3f342d8481a5a8e87c81  agent-results/riltra-events.jsonl
a289137dfa565292fb4841fb81b48ca2fed32b4110fa1057f9a87e60ca1568a5  results/a-callers.json
5ca7b43d74af951db5156171bfb225f7f8dd391c9c41c15b8da8dd328db7f1a7  results/b-callers.json
e25b1c71afcdd46ec1791ad6c7398b2d0531267fd85d11a89c603362741979af  results/a-impact.json
3de0e3701c240d5a01f53a3acdfe207793ac3a4fc3cc4fae408fdae3996f5ce1  results/mcp-raw.jsonl
cc4410e33a83c90b34af7d06f7ebb78746b8122ee7edfd5eae8e1661960f4f94  results/mcp-stale-raw.jsonl
```

Masih diperlukan oracle yang lebih luas dan ditinjau independen, lebih dari satu fixture/repository, kasus alias yang ambigu atau mutable, referensi lain, batas paging, dan pengulangan lintas lingkungan. Kasus ini tidak memverifikasi C++ atau seluruh pernyataan safety. Catatan sebelumnya: [correctness follow-up](riltra-correctness-followup-2026-11-02.md) dan [pilot lama fixture ini](pilots/wiy-caller-precision-2026-10-31/REPORT.md).
