# Bukti pilot agent dengan dan tanpa Riltra

Tanggal: 2 November 2026. Status: **pilot diagnostik; bukan bukti keunggulan umum**.

## Pertanyaan dan batas klaim

Pada satu tugas pencarian definisi, apakah dua sesi Codex lokal dapat menjawab dengan benar, dan apa waktu serta penggunaan token yang dilaporkan? Kelompok Riltra diminta menggunakan CLI; kelompok pembanding memakai pencarian sumber. Instruksi kedua kelompok berbeda, sehingga hasil mencakup pengaruh instruksi dan alur kerja, bukan efek murni engine. Hasil ini tidak mengukur kemampuan menyelesaikan perubahan kode atau penggunaan alami tanpa instruksi memakai Riltra.

## Metode

- Sumber: arsip `crates/cli/src` dari commit `1ac971b237b55fe09bf65fb1f9825e24ea043e66`; arsip SHA-256 `7e5792082714254c8fc793af4acfb84f33a724b7a7f272ca975b6cfd1cabd07f`. Masing-masing kelompok mendapat salinan sendiri.
- Tugas yang sama dalam setiap sesi: cari definisi lima fungsi dan laporkan path, nomor baris, serta offset byte nama. Pemeriksa menghitung lokasi secara independen dari byte sumber; expected M10 lama tidak dipakai karena commit berbeda.
- Codex CLI 0.166.0, Windows, `--no-daemon`, satu sesi baru per kondisi. Model efektif tidak tercatat pada log, sehingga identitas model yang sama tidak dapat dipastikan dari bukti. Urutan selalu baseline lalu Riltra. Waktu adalah stopwatch operator dari peluncuran sampai akhir pipeline, bukan pengukuran komponen terisolasi.
- Input token dan output token berasal dari event `turn.completed`; token cached termasuk dalam input token. Biaya uang tidak dihitung. Nilai bervariasi dengan konteks, cache dan model.
- Pengulangan menggunakan `workspace-write` pada kedua sesi; instruksi melarang perubahan sumber. Direktori indeks ada di dalam salinan Riltra. Seluruh 24 file sumber setiap salinan pengulangan dibandingkan dengan arsip dan identik setelah sesi; status Git repo utama bersih.
- Index Riltra dibangun sebelum sesi; `ingest` 21.549 ms, `verify` 181 ms, `promote` 264 ms menurut JSON index. Ini bukan stopwatch total indeks. Build binary, instalasi, dan biaya setup lainnya tidak diukur. Binary SHA-256 `215a1b4ac46e0d5063ae9d4be2e24ce3bcfead2b822dde249405609faf682a69`.

## Hasil

| Percobaan | Kondisi | Jawaban benar | Waktu (detik) | Input token | Input cached | Output token | Kondisi alat |
|---|---|---:|---:|---:|---:|---:|---|
| Awal | Native | 5/5 | 36,93 | 130.609 | 112.128 | 868 | `rg` tidak tersedia; agent memakai PowerShell |
| Awal | Riltra | 5/5 | 51,75 | 193.279 | 172.032 | 1.124 | Lima indexed search gagal membuka database; lima scan berhasil, lalu sumber diperiksa langsung |
| Ulang | Native | 5/5 | 31,48 | 105.913 | 97.152 | 678 | `rg` tidak tersedia; agent memakai PowerShell |
| Ulang | Riltra | 5/5 | 22,88 | 83.506 | 73.728 | 465 | Lima indexed search berhasil, `fresh`, tidak terpotong; agent memeriksa byte sumber |

Pada pengulangan saja, sesi Riltra mencatat waktu dan input token lebih rendah daripada sesi native pada tugas ini. Pada percobaan awal, arah perbandingannya terbalik dan indexed search gagal. Ini adalah empat pengamatan, bukan estimasi yang andal tentang performa umum atau sebab-akibat. Penyebab kegagalan membuka database pada sesi awal belum dipastikan; pemindahan direktori indeks ke workspace dan perubahan mode sandbox dilakukan bersamaan sehingga efeknya tidak dapat dipisahkan.

Ground truth pada arsip yang diuji:

```text
canonical_root crates/cli/src/paths.rs:57 bytes 1704-1718
contains crates/cli/src/paths.rs:72 bytes 2180-2188
resolve_recorded_root crates/cli/src/main.rs:185 bytes 8437-8458
escape crates/cli/src/json.rs:7 bytes 356-362
grammar_versions crates/cli/src/store.rs:27 bytes 1289-1305
```

## Keterbatasan dan audit

Hanya satu tugas yang diulang; hasil, urutan, dan beban cache tidak diacak. Sumber dan prompt berasal dari proyek yang dapat diketahui model, dan sesi native membaca skill dari luar salinan, sehingga isolasi tidak sempurna. Perbedaan akses Riltra dan instruksi memang disengaja, tetapi skill dan tool yang dipilih agent juga tidak sama. Tidak ada pengukuran jumlah biaya build, benchmark SWE, pengujian lintas model, atau confidence interval. Jangan memakai hasil sebagai klaim umum kecepatan, penghematan token, atau kualitas agen.

Bukti mentah disimpan di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/` dan diabaikan Git. Log memuat path lokal dan teks kode; tinjau dan redaksi sebelum dibagikan. Hash di bawah berlaku untuk bukti mentah tersebut:

```text
65d0301751f0a00a092e7c0f517060aca4c30ced19fac070605466d426f79d27  base-local-events.jsonl
b5483d615581d93a1abbec27891997828794ac04fc65b788b43bca3aeda19cf1  riltra-local-events.jsonl
4a11bbc456c363eba8452db396709d600c127579fb12de2a9a7fc0cc2611cff4  repeat-base-events.jsonl
dd20ae5b3b0f54b01c946d36f548f1f17f7488b9654b702d22d19a752a5a9f93  repeat-riltra-events.jsonl
1818768c11b08d7fe06f89dcce817aa669a2f3e2428888b2ec4ebddcd5e5ec0b  repeat-base-timing.txt
a7924760971a52557ad6ada4703e08bba570c81294f95037cba6e25d5b1a3353  repeat-riltra-timing.txt
```
