# Pengulangan fixture Rust dengan producer nyata

Tanggal 3 November 2026. Fixture alias, dua definisi bernama sama, dan referensi non-call dari `wiy-caller-precision-2026-10-31` disalin ke tiga source/home terpisah. Acuan sumber dicatat sebelum query, tetapi fixture dan hasil terdahulu sudah dikenal; run ini tidak buta dan oracle tidak direview independen untuk run ini.

`rust-analyzer 1.99.1` menghasilkan SCIP offline pada tiap source. Cargo target build di luar source tetapi berbagi cache; tiga home index terpisah tidak berarti tiga build dingin atau pengamatan statistik independen. Producer log yang disimpan tidak memiliki baris `ERROR` atau `WARN`; ini bukan audit seluruh diagnostics. Producer dapat menjalankan build script proyek tepercaya.

Pada setiap home, query pertama dan query kedua `callers` pada `a::target` mengembalikan dua `CALLS` (`a::target()` dan alias `f()`) serta dua `REFERENCES` (`target` pada initializer), target identity `a`, freshness `fresh` dan `Unverified`. Slice byte untuk seluruh enam respons diperiksa. Keenam query cocok dengan oracle terbatas; ini bukan enam fixture yang berbeda. Query `b::target` tidak diulang dalam run ini; hasil pengujian sebelumnya dicatat dalam [laporan fixture Rust](../../riltra-rust-alias-correctness-2026-11-03.md).

Bukti source, oracle, producer dan output disimpan lokal dengan hash salinan diverifikasi di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/rust-fixture-repeats/`, diabaikan Git dan dapat mengandung path perangkat. Tidak ada klaim waktu, biaya, generalisasi semua alias, atau provider availability di luar tiga run ini.
