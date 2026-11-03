# Pemeriksaan lanjutan kebenaran Riltra — 2 November 2026

Status: **parsial dan terbatas pada kasus di bawah**, bukan PASS produk secara keseluruhan. Pemeriksaan dilakukan di Windows pada HEAD `604cd7365a11cfaba625b4688e64bb16bb67982f`; kode dan Cargo lock identik dengan commit sumber arsip `1ac971b237b55fe09bf65fb1f9825e24ea043e66`. Binary yang dibangun dengan `cargo build --locked` SHA-256 `215a1b4ac46e0d5063ae9d4be2e24ce3bcfead2b822dde249405609faf682a69`. Snapshot source berasal dari arsip source-only pilot dan tidak memiliki `.git`; head dalam output Riltra kosong. Home dan output query berada di luar snapshot.

## Temuan occurrence pada pilot

Query langsung `search --occurrences` untuk `canonical_root` dan `MAX_JSON_DEPTH` pada snapshot fresh menghasilkan nol item dengan `binding: unbound`. Ini **bukan bukti bahwa referensi tidak ada**. Implementasi `FileFacts.reference_occurrences` mendeskripsikan fakta nama untuk sumber C++, dan [catatan E7](m9-e7-implementation.md) menjelaskan census tersebut sebagai fakta C++ yang tidak membuktikan binding. Karena kasus pilot menggunakan Rust, nol item tidak dapat dinilai sebagai false negative referensi Rust tanpa kontrak yang menjanjikan cakupan itu. README juga menyatakan daftar kosong tidak membuktikan nama tidak digunakan. Temuan ini menunjukkan keterbatasan yang relevan untuk pemilihan tool oleh agent, bukan bukti kerusakan yang sudah terdiagnosis.

## Callers syntax dan freshness

Oracle sumber pilot memuat 20 lokasi `canonical_root(...)` (termasuk satu di dalam macro). `callers --repo trial --path crates/cli/src/paths.rs --line 57 --col 8 --limit 100` mengembalikan 19 lokasi unik; 19 cocok dengan oracle, tidak ada lokasi tambahan, dan semua berlabel `POTENTIAL_CALL`/`Heuristic`. Lokasi yang tidak ditemukan adalah `crates/cli/src/paths.rs:122`, di dalam `assert!`. Pengukuran ini adalah penemuan lokasi menurut nama pada satu oracle, bukan bukti identitas 19 target, dan bukan recall semua pemanggil Rust. Hasil sendiri melaporkan `partial`, `Unverified`, dan anchor syntax tanpa identitas semantic.

`impact` pada anchor tersebut mengembalikan `UNKNOWN`, `partial`, tanpa path; hasil kosong itu tidak membuktikan aman. Setelah komentar ditambahkan **hanya pada salinan uji** `paths.rs`, `index status` dan `search` melaporkan `stale` dengan alasan `content-changed`. Ini satu pemeriksaan perubahan konten, bukan pengujian semua skenario freshness.

## Rust semantic dengan producer nyata

Fixture closure yang sudah ada disalin ke direktori terisolasi; `Cargo.toml` salinan diberi `[workspace]` dan lockfile dibuat offline. Source `src/lib.rs` SHA-256 `dace3a0e334081b80854925a300fc73d987cce62b704730a5bcae5c4124e77d3`. Acuan fixture: `direct_caller` memanggil `callee` langsung; panggilan kedua ada di closure tanpa owner yang terbukti.

`rust-analyzer 1.99.1` dijalankan secara eksplisit dengan `CARGO_NET_OFFLINE=true`, target terpisah dan `scip --exclude-vendored-libraries`; producer dapat menjalankan build script dari proyek tepercaya. Import melaporkan satu dokumen, tujuh definisi, 11 referensi, tanpa dokumen dilewati dan `Unverified`. File SCIP lokal SHA-256 `cacd0f5e746d13ab1585623c7d80332f0d3b8bb3f6a7b2e08eeb7c2115d314cb`; artifact dapat menyertakan path lokal. Tidak ada baris `ERROR` atau `WARN` pada log producer yang disimpan; ini bukan audit semantik menyeluruh.

Query `callers` pada `callee` mengembalikan dua lokasi: panggilan langsung sebagai `CALLS`/`Exact`/`Unverified`, dan closure sebagai `POTENTIAL_CALL`/`Heuristic`/`owner-unproven`. Ini cocok dengan dua kasus fixture dan kontrak konservatif; hasil tetap `partial`. `impact` mengembalikan `UNKNOWN`. Ini mengulang kasus yang sudah pernah diuji, bukan oracle baru untuk alias, tabrakan nama, trait atau seluruh repository.

## Tes integrasi

`cargo test --locked --test edges --test mcp --test semantic -- --quiet` pada Windows selesai exit 0: **158 edges, 61 MCP, 22 semantic** lulus. Ini memverifikasi assertions suite yang ada, termasuk kasus CLI/MCP dan fixture alias yang dicakup suite; tidak berarti seluruh kombinasi provider atau semua jawaban diperiksa manual. Tidak ada tes baru atau perubahan source produk.

## Batas dan pekerjaan tersisa

Belum ada evaluasi baru dengan oracle independen yang lebih besar untuk producer nyata pada alias, collisions dan non-call references, belum ada audit lengkap diagnostik producer repository penuh, dan belum ada perbandingan agent baru setelah hasil engine ini. Belum ada pengulangan lintas sistem operasi atau pemeriksaan menyeluruh terhadap setiap output CLI/MCP di luar assertions tes. Tidak ada angka rata-rata yang mewakili kebenaran produk atau klaim kecepatan/biaya.

Bukti query, fixture dan output producer disalin ke `docs/pilots/agent-kebenaran-2026-11-02/raw-local/correctness-followup/`, yang diabaikan Git; 18 file salinan diverifikasi terhadap aslinya dengan SHA-256. Arsip sumber pilot ada di direktori induk `raw-local/`. Bukti lokal dapat berisi path pribadi dan tidak tersedia bagi pembaca yang hanya memiliki commit; periksa dan redaksi sebelum membagikan. Catatan sebelumnya: [pilot agent](pilots/agent-kebenaran-2026-11-02/suite-lanjutan/laporan.md) dan [evaluasi correctness](riltra-wiy-correctness-evaluation-2026-10-31.md).
