# Run agent multibahasa: audit awal

Run pada commit `093dab8808fb353e49ce86c7723c54c7e3ea15e0` mencakup 17 fixture dan 85 sesi sesuai [metode](metode-agent.md). Pemeriksaan log menemukan 85 `turn.completed`, 85 exit code 0, tidak ada stderr non-kosong dan tidak ada event `turn.failed` atau `error`. Hash semua 51 salinan fixture setelah run cocok dengan hash yang dicatat skrip. Ini menunjukkan penyelesaian proses dan integritas fixture, bukan kebenaran jawaban.

Manifest dan perintah skrip meminta `gpt-6-luna`; pemeriksaan teks event tidak menemukan metadata model untuk mengonfirmasi model aktual. Log mencakup event perintah yang menyebut Riltra, tetapi hitungan event tidak membuktikan setiap sesi memakai alat dengan benar atau isolasi reviewer. Salinan 394 file bukti ke direktori lokal Git-ignored `docs/pilots/agent-kebenaran-2026-11-02/raw-local/multilang-20261103/agent-run/` diverifikasi dengan SHA-256. Bukti mengandung path lokal dan tidak ikut commit.

Pemeriksaan isi awal menemukan kesalahan yang konkret:

- `c-reference/riltra-1`: jawaban menempatkan referensi `target` pada baris 7, padahal sumber menempatkannya pada baris 6.
- `c-same_name/riltra-2`: jawaban menggunakan baris 1, 2, 4 untuk tiga kejadian, padahal berkas termasuk komentar lisensi dan kejadian berada pada baris 2, 3, 5.
- `ts-same_name/review-1`: reviewer menambahkan selector pada `obj.target()` sebagai referensi non-call terpisah. Apakah itu duplikasi bergantung pada aturan unit penilaian; oracle harus menetapkan bahwa suatu call-site tidak otomatis dihitung lagi sebagai non-call sebelum angka dibandingkan.

Jawaban baseline dan Riltra sering berbeda dalam tingkat kepastian: arm Riltra diminta memakai indeks syntax-only dan membedakan kandidat dari binding terverifikasi, sedangkan baseline diminta menarik kesimpulan dari kode. Ketidakpastian yang tepat tidak otomatis kesalahan, dan sumber yang dibaca agent dapat menyumbang pada jawabannya. Jangan menafsirkan penyelesaian 85 sesi sebagai 85 jawaban benar, atau sebagai pengukuran terisolasi akurasi engine Riltra.

Audit awal ini belum menilai seluruh isi dan tool call per sesi. Jawaban akhir selanjutnya dinilai dalam [audit 85 sesi](audit-85-sesi.md); oracle belum ditinjau independen dan tool call belum diaudit menyeluruh. Pertahankan seluruh sesi, termasuk kesalahan.
