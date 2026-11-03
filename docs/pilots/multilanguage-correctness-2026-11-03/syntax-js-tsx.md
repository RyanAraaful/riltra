# Pemeriksaan syntax JavaScript dan TSX

Tanggal 3 November 2026. Dua fixture kecil dibuat pada source terisolasi; acuan ditulis sebelum menjalankan scan tetapi belum ditinjau reviewer terpisah.

| Fixture | Acuan | Hasil scan CLI |
|---|---|---|
| JavaScript | definisi `target` dan `caller`; `target()` pada baris 2 | Kedua definisi dan satu call ditemukan; `errors: 0` |
| TSX | definisi `target` dan `View`; `target()` di dalam JSX pada baris 2 | Kedua definisi dan satu call ditemukan; `errors: 0` |

Run awal memeriksa ekstraksi syntax pada dua file kecil, bukan resolusi binding semantic atau cakupan seluruh bahasa. Tidak ada perbandingan MCP pada run tersebut. Bukti source, oracle dan output ada di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/js-tsx-check/`, disalin dengan verifikasi SHA-256; direktori tidak masuk Git dan dapat berisi path lokal.

## Pengujian producer semantic susulan

Salinan kedua fixture diberi konfigurasi `allowJs` dan `jsx: preserve`, lalu diproses menggunakan `scip-typescript 0.4.0`. Setiap import melaporkan satu dokumen, tanpa skipped dan `Unverified`. Query `callers` pada masing-masing `target` mengembalikan satu `CALLS` dengan span yang dislice ke `target()` pada baris 2, `fresh` dan completeness `partial`. Acuan call ditulis sebelum producer, tetapi belum ditinjau secara independen. Ini satu contoh per bahasa, bukan bukti cakupan seluruh JS/TSX, dan tidak ada parity MCP pada run ini. Output producer dan query disimpan lokal dengan verifikasi hash di `raw-local/multilang-20261103/js-tsx-semantic/`.
