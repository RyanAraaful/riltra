# Evaluasi multibahasa: hasil sementara 3 November 2026

Status **parsial**: laporan ini mencatat corpus sintetis, satu fixture provider TypeScript, dan tiga tugas agent Rust. Ia bukan evaluasi lengkap seluruh rencana atau klaim kebenaran seluruh produk. Commit kode yang diuji `59218fa23fbfc53919cac4358335a8daf0d83154`.

## Corpus dan cakupan

Corpus M6 `cargo test --locked --test edges task7::` lulus **82/82** pada Windows (Rust 1.99.1) dan pada Ubuntu WSL 2 (Rust 1.93.1, offline, pengguna tanpa root). [Matriks 79 kasus](matriks-m6.md) memetakan test dan manifest ke masing-masing hasil Windows/WSL. Terdapat 15 tes modul untuk masing-masing Python, TypeScript, Rust, C, C++, empat tes nested anonymous, serta tiga tes impact Rust tambahan. Manifest dan producer SCIP corpus sintetis, bukan keluaran indexer bahasa nyata; angka test lulus tidak membuktikan semua kasus semantic nyata. JavaScript dan TSX belum mendapat matriks tersendiri.

## TypeScript: producer nyata

Pada fixture existing `gsem/fixture/ts`, `scip-typescript 0.4.0` diimport dengan dua dokumen, empat definisi, enam referensi, nol skipped dan `Unverified`. Expected sumber: import `greet` pada `src/main.ts:1`, pemanggilan pada `:3`. Tiga home dan producer terpisah serta dua query per home (cold/warm) semuanya menghasilkan kedua referensi (6/6 query), tanpa fallback; ini bukan enam fixture independen. Oracle tidak direview secara buta pada run ini.

Pada satu run, `callers` hanya mengembalikan `REFERENCES` pada import dan tidak menampilkan call baris 3; hasil `partial`, `Unverified`, dan alasan completeness mencakup `unresolved_owner`. Pemanggilan berada di scope modul; keterkaitan pasti dengan alasan tersebut belum didiagnosis. Satu respons MCP `riltra_code_callers` identik dengan JSON CLI. Perbedaan cakupan semantic reference dan edge callers ini perlu diagnosis terhadap kontrak owner/anchor sebelum disebut defect. Tidak diuji pada repository TypeScript nyata dalam run ini.

Pemeriksaan `PATH` pada Windows menemukan `rust-analyzer` dan `scip-typescript`, tetapi tidak menemukan `clangd`, `scip-clang`, `pyright-langserver`, `pylsp`, atau `typescript-language-server`. Ini hanya inventaris PATH, bukan bukti bahwa program tersebut tidak terpasang di lokasi lain, dan bukan pengukuran provider C/C++ atau Python.

## Agent dan review acuan

Skrip terminal menyiapkan source archives identik dan menjalankan masing-masing satu sesi reviewer, baseline dan Riltra untuk A1, A2 dan A3. Seluruh sesi mencapai `turn.completed`. Pemeriksaan operator setelahnya membandingkan baris jawaban terhadap source: setiap kelompok mencocokkan 5/5 definisi dan span pada A1, 27/27 call-site pada A2, dan 3/3 fakta konstanta pada A3, tanpa entri tambahan yang terdeteksi. Reviewer memakai sesi terpisah tetapi isolasi akses tidak dibuktikan secara teknis dan prompt serta fixture telah dipublikasikan sebelumnya.

Agent Riltra melakukan lima indexed searches pada A1, tetapi lima percobaan semantic query gagal; agent memeriksa byte sumber. Pada A2 agent memanggil search dan callers serta memakai pencarian teks sumber; pada A3 query occurrence tidak cukup dan agent juga mencari sumber. Dengan demikian 3/3 jawaban akhir bukan bukti bahwa Riltra sendiri menyediakan semua fakta. Satu sesi per kondisi dan tiga tugas Rust yang sama tidak mendukung kesimpulan umum atau klaim kecepatan/biaya.

## Bukti dan sisa pekerjaan

Output corpus, agent dan TypeScript mentah disalin dengan pemeriksaan SHA-256 ke `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/`, diabaikan Git karena memuat path lokal. Tidak disertakan dalam commit; akses kepada orang lain memerlukan sanitasi terpisah. Salinan source agent: 24 file masing-masing dibandingkan dengan arsip dan identik setelah sesi.

Belum selesai: oracle per kasus tambahan yang dibutakan dan direview terpisah; producer nyata Rust/TypeScript pada repository nyata baru; provider lain, cold/warm seluruh bahasa, parity MCP per kasus termasuk stale/error/paging, perbandingan agent multibahasa dengan pengulangan dan urutan bergantian, serta pengukuran ketahanan. Hindari menggabungkan tes yang belum dinilai ke dalam rata-rata. Laporan ini mempertahankan gap yang ditemukan tanpa mengubah hasil historis.
