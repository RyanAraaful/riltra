# Hasil suite lanjutan: kebenaran agent dan penggunaan Riltra

Tanggal 2 November 2026. Status: **pilot terbatas**, bukan ukuran kebenaran universal atau benchmark statistik.

## Hasil yang dapat diklaim

Pada tiga tugas di bawah, satu sesi agent tanpa Riltra dan satu sesi yang diminta memakai Riltra masing-masing memberikan jawaban yang sesuai sumber: **3/3 tugas per kondisi**. Dengan bobot sama per tugas, rata-rata ketepatan penyelesaian yang diamati adalah **100% (3/3)** untuk masing-masing kondisi. Ini bukan probabilitas keberhasilan di luar sampel dan bukan bukti bahwa seluruh fakta diperoleh dari Riltra.

| Tugas | Acuan | Tanpa Riltra | Dengan Riltra | Bukti penggunaan Riltra |
|---|---|---|---|---|
| A1: definisi, baris dan offset byte | Lima definisi | 5/5 benar | 5/5 benar | Lima query indeks berhasil dan `fresh`; agent juga membaca byte sumber |
| A2: lokasi pemanggilan | 20 `canonical_root`, 7 `resolve_recorded_root` | 27/27 benar, tanpa tambahan | 27/27 benar, tanpa tambahan | Dua query dengan `--occurrences` berhasil tetapi masing-masing mengembalikan nol item; agent memakai pencarian teks sumber untuk daftar akhir |
| A3: nilai konstanta dan pembaca | `64`, baris 364 dan 380 | 3/3 fakta benar | 3/3 fakta benar | Query indeks mengembalikan nol definisi dan nol occurrence; agent memakai pencarian teks sumber |

Secara deskriptif, setiap kondisi menghasilkan **35/35 fakta acuan** (5 + 27 + 3) tanpa tambahan yang salah pada jawaban akhir. Fakta dalam kategori yang berbeda diberi bobot satu untuk hitungan ini; angka tersebut tidak menunjukkan 35 tugas independen. Penilaian membandingkan substansi; tanda spasi penutup atau urutan tidak mengubah nilai.

## Metode dan acuan

Enam sesi independen: tiga prompt A1/A2/A3 dari `docs/pilots/m10/freeze/prompts`, masing-masing baseline kemudian Riltra. Pengulangan memakai salinan `crates/cli/src` dari commit `1ac971b237b55fe09bf65fb1f9825e24ea043e66`, arsip SHA-256 `7e5792082714254c8fc793af4acfb84f33a724b7a7f272ca975b6cfd1cabd07f`. A3 memakai sumber saat ini **tanpa patch historis**: nilai acuan 64. Kedua kondisi memakai Codex CLI 0.166.0, `--no-daemon`, `workspace-write`, dan instruksi tidak mengubah file. Riltra 0.1.0 binary SHA-256 `215a1b4ac46e0d5063ae9d4be2e24ce3bcfead2b822dde249405609faf682a69`, indeks disiapkan sebelumnya; indeks saat dipakai dilaporkan `fresh`.

Acuan dibuat dari inspeksi sumber yang terpisah dari jawaban agent: pencarian definisi dan byte untuk A1, pemeriksaan 27 baris panggilan dan pengecualian definisi untuk A2, serta pencarian seluruh `MAX_JSON_DEPTH` untuk A3. Kedua salinan dibandingkan kembali terhadap arsip: 24 file sumber pada masing-masing salinan identik. Status Git repo utama bersih.

Acuan A1:

```text
canonical_root crates/cli/src/paths.rs:57 bytes 1704-1718
contains crates/cli/src/paths.rs:72 bytes 2180-2188
resolve_recorded_root crates/cli/src/main.rs:185 bytes 8437-8458
escape crates/cli/src/json.rs:7 bytes 356-362
grammar_versions crates/cli/src/store.rs:27 bytes 1289-1305
```

Acuan A2: `canonical_root` di `knowledge_binding.rs` 553, 1107, 1468; `knowledge_store.rs` 84, 665, 4828, 5465, 8543; `main.rs` 86, 157, 201, 467, 525; `mcp.rs` 515, 869; `paths.rs` 99, 110, 118, 122, 129. `resolve_recorded_root` di `main.rs` 222, 680, 800, 847, 882, 929, 1130. Semua path berawalan `crates/cli/src/`.

Acuan A3: definisi `crates/cli/src/knowledge.rs:286` bernilai `64`; dua pembaca pada baris 364 dan 380.

## Interpretasi dan batas

Hasil ini membuktikan kesesuaian jawaban akhir agent pada tiga tugas tersebut, dan bahwa sesi Riltra memanggil CLI serta mendapat respons indeks. Hasil ini **belum membuktikan** bahwa Riltra sendiri mengembalikan semua lokasi pemanggil atau pembaca pada A2/A3. Query occurrence menyatakan `binding: unbound` dan mengembalikan nol item; jangan menganggapnya daftar referensi lengkap atau otomatis menyebutnya bug tanpa membandingkan kontrak dukungan bahasa dan jenis fakta. Jawaban akhir A2/A3 ditemukan melalui pencarian sumber biasa. A1 juga diverifikasi lewat byte sumber, sehingga hasil agent tidak memisahkan kontribusi engine secara kausal.

Hanya tiga tugas, satu percobaan per kondisi, satu snapshot dan satu perangkat; model efektif tidak tercatat dalam event. Prompt kelompok Riltra mewajibkan alat dan kedua kelompok dapat membaca sumber; beberapa sesi membaca skill di luar salinan. `rg` tidak tersedia bagi agent, sehingga agent menggunakan PowerShell. Urutan tidak diacak. Karena itu, jangan menggeneralisasi persentase ini atau menarik klaim kecepatan, biaya, keamanan perubahan, kelengkapan referensi, atau keberhasilan SWE.

## Bukti mentah lokal

Log disimpan di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/suite-lanjutan/` dan diabaikan Git. Log dapat memuat path pribadi dan kode; tinjau sebelum membagikan. SHA-256 log mentah per sesi:

```text
cb4dfc204c6f8d25158e4d15c6a265c9cb387edd4239c21fa8ec05efc3069e4e  A1-base-events.jsonl
a1fbc9d3627df8190b5e3571f63a656ce060f15d3dba946999022d944f854a18  A1-riltra-events.jsonl
2ab141966cef630fe47f0000857c3345c8901bec3d8d88e0c9edb38c86aaf70b  A2-base-events.jsonl
aa0386c3a956ad9f26c0fbd5277a532b4100af57ffff478beaec0236da76df70  A2-riltra-events.jsonl
9b474c037fd9aa54e90e4e3d26a9112ed3e9c0d348a4927d6bea2ac502499039  A3-base-events.jsonl
9b7df99fac3722580d6c2020f391dc44051f33262d8e77e22b00e2e23a533c25  A3-riltra-events.jsonl
```
