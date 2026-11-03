# Baseline awal evaluasi multibahasa — 3 November 2026

Status: **baseline awal, pekerjaan evaluasi belum selesai**. Commit yang diuji `59218fa` pada Windows dengan cargo/rustc 1.99.1. Rencana: [evaluasi multibahasa](riltra-multilanguage-evaluation-plan-2026-11-03.md).

## Pemeriksaan lokal

| Perintah | Hasil |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --locked -- --quiet` | exit 0; 831 lulus, 0 gagal: 457 unit, 16 CLI, 158 edges, 44 index, 61 knowledge, 61 MCP, 22 semantic, 12 syntax |

Tes dijalankan di luar sandbox yang sebelumnya menolak operasi Git/hard link. Output terminal tidak diarsipkan sebagai log; ini catatan operator, bukan bukti mentah yang dapat diaudit ulang. Tidak ada perubahan source atau golden yang disengaja.

## Inventaris corpus

`crates/cli/tests/edges.rs` memuat `m6_corpus/mod.rs` sebagai modul `task7`. `cargo test --locked --test edges -- --list` mencatat 82 tes bernama `task7::`; tes-tes tersebut termasuk dalam 158 tes edges yang lulus. Pencarian awal memakai filter `m6_corpus` dan menghasilkan nol tes karena nama modulnya `task7`; itu bukan keberhasilan corpus dan tidak digunakan sebagai bukti.

Corpus historis menjelaskan 79 pasangan fixture dan 494 respons pada kelompok Python, TypeScript, Rust, C dan C++. Manifest SCIP adalah sintetis dan assertions memeriksa fakta source, label, impact dan golden; passing tidak membuktikan kualitas semua producer nyata. JavaScript dan TSX belum dinilai sebagai kelompok terpisah di inventaris ini. Angka respons historis belum dihitung ulang secara independen pada run ini.

## Pengulangan corpus dengan log

`cargo test --locked --test edges task7::` dijalankan ulang di Windows: 82 lulus, 0 gagal. Log menunjukkan masing-masing 15 tes di modul `py`, `ts`, `rs`, `c`, dan `cpp`, ditambah tujuh tes `task7` lain. Ini adalah hitungan tes, bukan klaim 82 fixture bahasa independen.

Perintah yang sama dengan `--offline` dijalankan di Ubuntu WSL 2 menggunakan cargo/rustc 1.93.1 dan pengguna tanpa hak root: 82 lulus, 0 gagal. Kedua run menggunakan checkout saat ini dan fixture SCIP sintetis. Log lokal ada di `target/multilang-20261103/windows-corpus.log` dan `wsl-corpus.log`; direktori target diabaikan Git dan dapat dibersihkan, sehingga log belum menjadi bukti portabel. Output dapat mengandung path lokal dan perlu ditinjau sebelum dibagikan.

## Producer TypeScript nyata: satu fixture

Fixture existing `docs/pilots/m9-rc-ee7482d/gsem/fixture/ts/` disalin ke home uji terisolasi; sebelum query, acuan sumber dicatat: definisi `greet` pada `src/greet.ts:1`, import pada `src/main.ts:1`, dan pemanggilan pada `src/main.ts:3`. Fixture dan hasil historis sudah dikenal; acuan ini tidak ditinjau secara independen atau buta untuk run ini.

`scip-typescript 0.4.0` menghasilkan artifact yang diimport sebagai dua dokumen, empat definisi, enam referensi, tanpa skipped, `Unverified`. Query semantic pada definisi menemukan dua referensi di `src/main.ts` pada baris 1 dan 3 (output semantic menggunakan koordinat baris berbasis nol). Query `callers` melaporkan satu `REFERENCES` pada import, freshness `fresh`, completeness `partial`, `Unverified`; lokasi pemanggilan baris 3 tidak muncul sebagai edge `CALLS` dalam query tersebut. Ini gap observasi untuk fixture dan query itu, bukan bukti cacat umum atau pelanggaran kontrak tanpa diagnosis owner/anchor. Tidak ada kesimpulan bahwa import yang tidak terverifikasi membuktikan binding runtime.

Satu query `riltra_code_callers` MCP dengan parameter anchor dan limit yang sama menghasilkan `structuredContent` identik dengan JSON CLI. Ini memeriksa satu kasus fresh, bukan seluruh parity MCP atau perilaku stale pada fixture ini.

Salinan 18 file bukti Windows/WSL dan fixture TypeScript telah diverifikasi SHA-256 terhadap aslinya dan disimpan lokal di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/`, diabaikan Git. Log dan artifact dapat mengandung path lokal dan tidak tersedia dari commit.

## Pekerjaan tersisa

Belum ada oracle baru yang ditinjau sesi terpisah, matriks per kasus, pemeriksaan producer nyata multibahasa, atau perbandingan agent baru. WSL di atas hanya mengulang corpus sintetis, bukan seluruh rencana. Hasil ini bukan klaim kebenaran atau optimalitas menyeluruh. Lanjutkan dengan inventaris per kasus dan oracle sebelum menjalankan query tambahan; simpan output mentah yang disanitasi dan hasil unavailable.
