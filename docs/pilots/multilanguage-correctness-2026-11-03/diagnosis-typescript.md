# Pemeriksaan owner pemanggilan TypeScript

Tanggal 3 November 2026. Status: diagnosis terbatas; belum ada perubahan produk. Temuan awal dan batas producer dicatat di [laporan evaluasi](laporan.md).

Fixture asli memanggil `greet` pada scope modul (`console.log(greet("world"))`). Semantic query menemukan referensi pemanggilan, tetapi `callers` mengembalikan referensi import saja dan completeness `partial` dengan `unresolved_owner`.

Sebagai pembanding, fixture terisolasi dibuat dengan import yang sama dan satu fungsi bernama `caller` yang memanggil `greet("world")`. Acuan ditulis sebelum menjalankan producer: import baris 1, panggilan pada baris 3. Acuan belum ditinjau reviewer independen. `scip-typescript 0.4.0` menghasilkan dua dokumen, lima definisi dan empat referensi; import `Unverified` dan tanpa skipped. Query `callers` pada definisi `greet` mengembalikan `REFERENCES` pada import dan `CALLS` pada `greet("world")`, fresh, partial dan `Unverified`. Slice sumber untuk kedua span diperiksa.

Ini konsisten dengan batas bukti owner: source `edges.rs` mensyaratkan owner callable yang terbukti bersama definisi target dan reference overlap sebelum mengeluarkan `CALLS`; ketika owner tidak ada, turunan yang dimaksud adalah kandidat `owner-unproven`. Namun, pembanding ini mengubah konteks pemanggilan dan output belum menelusuri setiap tahap query pada fixture pertama; hasilnya mendukung hipotesis, **belum membuktikan akar penyebab** mengapa kandidat tersebut tidak terlihat dalam respons `callers`. Jangan mengklaim seluruh panggilan scope modul tercakup atau mengubah kontrak berdasarkan satu kasus.

## Pemeriksaan counterfactual pada salinan database

Pemeriksaan read-only database asli menemukan edge `POTENTIAL_CALL` untuk `greet` pada byte `[46,60]` di `src/main.ts`, dengan `owner-unproven` dan callee unknown. Fakta definisi semantic untuk identitas `greet` memiliki `display_name` berupa string kosong. `resolve_edge_anchor` menyalin nilai tersebut, dan seleksi kandidat `callers` memakai nama anchor untuk pencocokan.

Database home disalin; pada salinan saja, `display_name` tepat satu fakta definisi `greet` diganti menjadi `greet` melalui SQLite. Query yang sama kemudian mengembalikan referensi import **dan** kandidat `POTENTIAL_CALL` pada `[46,60]` dengan `owner-unproven`. Database asli diperiksa lagi dan tetap memiliki nama kosong. Ini mendukung secara langsung bahwa nama tampilan kosong berkontribusi terhadap kandidat yang tidak tampak pada fixture ini; counterfactual memodifikasi data indeks secara manual dan **bukan** run producer sah atau pembuktian perbaikan produk. Upaya pertama memilih tiga baris, dihentikan assertion sebelum commit SQLite; transaksi rollback. Upaya kedua memilih satu identitas dan dicatat di sini.

Sebelum mengusulkan fix, tambahkan reproduksi regression yang menggunakan artifact producer dengan display name kosong, tinjau aturan fallback nama agar tidak mengubah identity atau mempromosikan kandidat menjadi `CALLS`, dan uji tabrakan nama serta batas kandidat. Output counterfactual dan salinan database disimpan secara lokal di `raw-local/multilang-20261103/real-ts/`; keduanya tidak masuk Git.

Bukti mentah, termasuk fixture, registry, indeks, artifact dan output, disalin dan diverifikasi hash secara lokal di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/ts-owner-control/`. Direktori diabaikan Git dan dapat mengandung path lokal; bukti tidak ikut commit. Kasus fixture kecil ini tidak membuktikan perilaku pada proyek TypeScript nyata, runtime, atau bahasa lain.

## Perubahan susulan

Tes regresi dengan SCIP sintetis mereproduksi kandidat yang hilang ketika display name kosong. Query kini dapat memakai teks span yang hanya berisi karakter identifier sebagai nama kandidat jika digest source cocok dengan fakta; identitas semantic dan display name untuk knowledge binding tetap berasal dari producer. Nama ini hanya dipakai untuk memilih kandidat, bukan membuktikan target atau menaikkan `POTENTIAL_CALL` menjadi `CALLS`. Tes tambahan memeriksa file lain dengan nama sama dan keadaan stale. Pada indeks fixture yang dihasilkan producer nyata sebelumnya, query susulan mengembalikan referensi import serta kandidat scope modul `owner-unproven`; hasil tetap `Unverified`. Ini belum membuktikan dukungan seluruh bentuk identifier atau semua producer.
