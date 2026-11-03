# Producer Rust pada salinan repository

Tanggal 3 November 2026. Satu run pada archive source commit `093dab8808fb353e49ce86c7723c54c7e3ea15e0`; archive mencakup workspace manifest, lockfile dan `crates/cli`, bukan seluruh repository. Source tidak memiliki `.git`. Archive SHA-256 `43bae5020fbf1a2d7e381b7b566739fee737d8d72ef06f869f2980e153bdd34a`.

Oracle sebelum query menggunakan kembali [review sumber terdahulu](../../riltra-wiy-full-project-recheck-2026-10-31.md): 16 ekspresi pemanggilan `confined_read`, termasuk closure dan assertion macros. Ini bukan oracle baru atau buta; source kode dan Cargo tidak berbeda dari commit `1ac971b` yang menjadi dasar pilot. Tidak mengukur seluruh simbol atau pemanggilan tidak langsung.

`rust-analyzer 1.99.1` dijalankan offline dengan `--exclude-vendored-libraries` dan Cargo target terpisah dari source, tetapi berbagi cache build repository; producer menjalankan build script dan proc-macro dependency. Import melaporkan 36 dokumen, 18.204 definisi, 106.314 referensi, nol skipped dan `Unverified`. Artifact SHA-256 `238429247ba252649b4eb6f053e98d4cb964f76b1fbf2aea508d8884f64880f4` dapat mengandung path lokal.

Index cold melaporkan 314 file ditemukan, 130 diproses, 184 unsupported dan satu syntax error; waktu ingest yang dilaporkan 35.459 ms. Ini data satu run, tidak membuktikan performa umum atau kelengkapan semua source. Database setelah import berukuran 75.501.568 byte; angka itu mencakup semantic import, bukan hanya indeks cold.

Query `callers` pada `crates/cli/src/semantic.rs:276:8` mengembalikan 16 lokasi oracle dan tidak ada lokasi tambahan: 12 `CALLS`, satu `POTENTIAL_CALL` (`owner-unproven`), dan tiga `REFERENCES`. Dengan semesta oracle yang dibatasi ini, penemuan lokasi adalah 16/16; 12 baris `CALLS` sesuai source, tetapi tiga reference dan satu kandidat tidak boleh dihitung sebagai call-edge terbukti. Respons fresh, tidak terpotong, dan `Unverified`. `structuredContent` MCP untuk callers identik dengan JSON CLI pada run ini.

`impact` menghasilkan `UNKNOWN`, `partial`, dan `truncated: true`, termasuk `time-budget` dan `unresolved-owner`; daftar path tidak diperiksa satu per satu. Ini bukan affected set lengkap atau klaim bahwa perubahan aman.

Producer exit 0 tetapi log berisi 197 baris ERROR: 191 `Encountered enclosing definition with no name` dan enam masalah definisi di dokumen SCIP. Producer console mencatat 93 lokasi duplicate symbol. Ini inventaris pesan, bukan audit dampaknya terhadap setiap simbol; tidak dapat disimpulkan bahwa seluruh import benar atau semua 197 diagnostik memengaruhi anchor. Hasil import tidak membawa seluruh diagnostik tersebut.

Bukti mentah source archive, oracle, artifact, log producer, output CLI/MCP dan status disalin secara lokal dan diverifikasi hash ke `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/real-rust-repo/`, yang diabaikan Git dan dapat mengandung path lokal. Tidak ada klaim bahwa bukti tersebut tersedia dari commit.

Setelah run, seluruh 314 file reguler dalam archive dibandingkan byte-per-byte dengan file snapshot dan tidak ada yang berubah atau hilang. Pemeriksaan ini tidak menghitung file baru di luar archive.
