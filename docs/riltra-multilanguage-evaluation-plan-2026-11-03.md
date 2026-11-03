# Rencana evaluasi multibahasa Riltra

Tanggal rencana: 3 November 2026. Status per 4 November: **sebagian sudah dieksekusi; evaluasi keseluruhan masih parsial**. Lihat [laporan multibahasa](pilots/multilanguage-correctness-2026-11-03/laporan.md), [hasil agent v2](pilots/multilanguage-correctness-2026-11-03/agent-v2-results.md), dan [hasil tindak lanjut](pilots/multilanguage-correctness-2026-11-03/hasil-tindak-lanjut.md).

Yang sudah dilakukan: baseline corpus dan pengulangan Windows/WSL; pemeriksaan engine/provider terbatas pada fixture dan subset repository nyata; satu pemeriksaan freshness/incremental CLI–MCP; serta 68 sesi perbandingan agent v2 pada 17 fixture. Banyak kriteria rinci di bawah belum dipenuhi. Hasil agent v2 memakai fixture yang sudah dikenal dari v1, akses file tidak dibatasi secara teknis, dan bukan ukuran akurasi produk atau benchmark proyek nyata.

Rencana ini tidak memberi persetujuan biaya benchmark eksternal atau publikasi klaim baru.

**Tujuan:** menentukan kebenaran fakta, cakupan kemampuan, keandalan penggunaan, dan prioritas pengembangan berdasarkan bukti per bahasa.

**Pendekatan:** gunakan corpus dan harness existing terlebih dahulu. Uji engine langsung terhadap oracle sumber sebelum membandingkan agent. Setiap tahap menghasilkan laporan yang dapat ditinjau sendiri; perubahan produk hanya dilakukan setelah cacat terdiagnosis.

**Perkakas:** stable Rust, CLI/MCP Riltra, harness existing, provider semantic opt-in. Windows sebagai lingkungan awal; Linux/WSL sebagai reproduksi terpisah. Jangan menambahkan dependency atau framework benchmark sebelum kebutuhan terbukti.

## 1. Inventaris dan baseline terpin

- [ ] Catat commit, binary SHA-256, lockfile, OS, toolchain dan provider. Buat source snapshot, home, build dan output terpisah di direktori scratch baru.
- [ ] Baca `docs/m6-task7-validation.md`, `docs/m6-validation.md`, `crates/cli/tests/fixtures/m6/README.md`, dan test target `edges`. Temukan perintah corpus yang benar dari implementasi saat ini; jangan mengasumsikan angka historis masih sama.
- [ ] Petakan kasus existing menurut bahasa, kemampuan, oracle sintetis/producer nyata, dan assertions yang benar-benar diperiksa.
- [ ] Jalankan `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, dan `cargo test --locked`. Simpan perintah, exit code dan ringkasan. Bedakan kegagalan lingkungan dari correctness; semua percobaan dipertahankan.

Hasil: manifest versi dan inventaris cakupan. Angka historis 79 kasus/494 respons adalah referensi awal, bukan hasil run ini.

## 2. Suite engine multibahasa

Bahasa awal: Python, JavaScript, TypeScript/TSX, Rust, C, C++. TSX dicatat sebagai varian tersendiri dalam tabel cakupan. CUDA menjadi kasus unsupported yang diharapkan.

- [ ] Gunakan ulang corpus M6 dan goldens. Jalankan query dan periksa source span, identitas, provenance, freshness serta batas certainty, bukan hanya exit code.
- [ ] Pilih setidaknya tiga keluarga kasus per bahasa dari fixture existing: definisi/identitas bernama sama; panggilan versus referensi/alias bila relevan; perubahan source dan keterbatasan evidence. Catat variasi bahasa yang tidak cocok dan alasan, jangan memaksakan fitur yang tidak didukung.
- [ ] Sebelum query tambahan, tulis expected dari source dan minta sesi pemeriksa terpisah meninjaunya tanpa output Riltra. Catat akses konteks awal, kebocoran, dan status review. Ketidaksetujuan diselesaikan dari sumber dengan amendemen bertanggal.
- [ ] Untuk setiap kasus, catat expected, observed, TP/FP/FN pada universe yang didefinisikan, unresolved, unsupported, truncation dan hasil validasi span. Candidate yang jujur tidak otomatis menjadi fakta salah; kandidat juga tidak dihitung sebagai binding terbukti.
- [ ] Bandingkan CLI dengan MCP pada kasus yang sama, termasuk error/missing-index, hasil kosong, paging, fresh dan stale. Validasi `structuredContent` terhadap dokumen CLI dengan perbedaan transport yang dijelaskan.

Kriteria penilaian: PASS hanya untuk assertions kasus yang diperiksa. Fakta/target palsu, kepastian yang tidak didukung, stale yang disajikan sebagai fresh, atau keamanan perubahan yang diklaim tanpa evidence menjadi temuan correctness. Nol hasil harus dinilai terhadap kontrak bahasa; bukan otomatis bukti tidak ada referensi.

## 3. Provider nyata dan repository nyata

- [ ] Pilih producer yang tersedia dan berlisensi sesuai, dimulai dari Rust dan TypeScript yang sudah memiliki bukti fixture. Untuk Python, JS, C dan C++, catat feasibility/provider sebagai gate, bukan menyamakan import contract dengan dukungan semantic terverifikasi.
- [ ] Bekukan repository, commit, lisensi dan subset anchor sebelum mengukur. Mulai dengan satu repository per keluarga provider yang tersedia; tambah repository kedua setelah prosedur dapat direproduksi.
- [ ] Tetapkan subset source yang dapat diaudit: definisi, direct calls, alias, collision, non-call reference serta unresolved dispatch yang relevan. Jangan menghitung recall seluruh repository tanpa oracle lengkap.
- [ ] Jalankan provider hanya pada source tepercaya dengan persetujuan opt-in. Catat build script/procedural macro, konfigurasi, diagnostics, dokumen dilewati, versi dan hash artifact. Offline tidak berarti kode tidak dieksekusi.
- [ ] Ulangi cold dan warm pada home terpisah untuk membedakan kebenaran output, availability provider dan transparansi kegagalan. Audit diagnostik yang memengaruhi anchor; hasil keluar 0 saja bukan bukti benar.

Hasil: tabel kemampuan per bahasa/provider dan kasus, termasuk yang belum tersedia. Pertahankan `Unverified`, `partial`, dan `UNKNOWN` sesuai kontrak.

## 4. Freshness dan ketahanan

- [ ] Gunakan suite existing untuk edit, rename, delete, perubahan branch, incremental versus full, limit deterministik dan missing provider. Periksa cakupan sebelum menambah kasus.
- [ ] Pada source copy, ukur index cold, incremental, ukuran DB dan penggunaan memori bila alat pengukur tersedia. Pertahankan kebenaran sebagai syarat; catat ukuran source, jumlah fakta, cache dan setup.
- [ ] Ulangi kasus representatif pada WSL/Linux, sambil memisahkan versi toolchain, filesystem dan hak root. Tes izin file harus memakai pengguna tanpa hak root.

Pengukuran resource dipakai mencari bottleneck dan kegagalan nyata; tidak menghasilkan klaim publik lebih cepat/hemat dari sampel kecil.

## 5. Agent dan benchmark eksternal

- [ ] Pilih tugas dari kasus engine yang sudah dinilai; bekukan oracle dan instruksi sebelum sesi. Agent baseline dan Riltra mendapat source identik serta model/versi, hak tool dan batas sesi yang dicatat. Catat bila model efektif tidak bisa diverifikasi.
- [ ] Uji kedua kondisi pada kasus sama, ulangi beberapa kali dengan urutan bergantian. Tentukan jumlah pengulangan dan batas biaya sebelum run, simpan setiap kegagalan dan sesi yang tidak tersedia.
- [ ] Nilai jawaban akhir terhadap oracle; audit tool call dan bedakan fakta Riltra, verifikasi source dan fallback native. Melihat agent benar tidak otomatis membuktikan keluaran engine lengkap.
- [ ] Setelah tahap engine, audit kelayakan subset SWE-bench/SWE-bench Pro: versi, bahasa yang didukung, lisensi, biaya dan kebutuhan container. Download dan eksekusi eksternal menunggu keputusan scope/biaya konkret. Tidak ada skor leaderboard resmi dari subset terkurasi.

Prioritas agent adalah correctness dan penggunaan evidence, tanpa target klaim kecepatan atau penghematan token.

## 6. Pelaporan dan keputusan pengembangan

Gunakan folder baru `docs/pilots/multilanguage-correctness-<tanggal-run>/`. Simpan laporan, manifest kasus dan bukti yang telah disanitasi. Log pribadi dan artifact berpath lokal tetap di penyimpanan lokal yang diabaikan Git; dokumentasikan lokasi dan hash serta ketersediaannya bagi pembaca commit.

- [ ] Tabel per kasus: bahasa/provider, pin, origin/review oracle, expected/observed, TP/FP/FN, candidate/unresolved/unsupported, freshness, parity CLI–MCP, status dan pointer bukti.
- [ ] Ringkas per bahasa dan kemampuan sebelum rata-rata gabungan. Cantumkan denominator, status unavailable, dan kasus yang belum diuji. Jangan gabungkan unsupported dengan salah, atau menampilkan rata-rata tanpa cakupan.
- [ ] Prioritaskan: fakta salah/false certainty dahulu; lalu silent stale, kegagalan akses/provider dan kesalahan pemilihan tool; baru kelengkapan capability dan bottleneck yang terukur.
- [ ] Setiap usulan fix memuat reproduksi, akar masalah, kontrak/caller terdampak, regression test dan validasi OS yang relevan. Jangan mengubah kontrak hanya agar skor naik.
- [ ] Periksa tautan, hash dan sanitasi; jalankan Gitleaks 8.30.1 sebelum commit DCO. Jangan mengubah freeze atau hasil historis untuk menyesuaikan run baru.

Prioritas berikutnya: buat oracle baru yang ditinjau sebelum output untuk proyek atau fixture yang belum dipakai; perluas pemeriksaan producer nyata dan parity CLI–MCP; lalu ukur freshness, kegagalan provider dan ketahanan pada kasus yang ditentukan. Evaluasi benchmark eksternal tetap menunggu scope, lisensi dan biaya yang konkret. Rencana ini tidak menjanjikan satu angka yang membuktikan seluruh kode sudah optimal.
