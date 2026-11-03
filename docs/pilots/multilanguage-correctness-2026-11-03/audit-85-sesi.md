# Audit 85 jawaban agent multibahasa

Audit operator pada 3 November 2026 membandingkan jawaban akhir setiap sesi dengan salinan source per kasus. Ini adalah penilaian pasca-run, bukan oracle yang dibutakan atau ditinjau independen. Unit penilaian adalah satu jawaban lengkap; dua pengulangan pada fixture yang sama bukan sampel independen. Reviewer di sini adalah sesi agent pada protokol, bukan pemeriksa independen yang aksesnya telah dibatasi secara teknis.

Aturan yang digunakan: semua lokasi, jenis kejadian dan fungsi pemuat yang relevan harus benar; baris dimulai dari 1 dan komentar lisensi ikut dihitung. Definisi, deklarasi field, call dan referensi non-call dibedakan. Selector yang menjadi bagian dari call tidak dihitung lagi sebagai referensi non-call terpisah. Keterangan tambahan tentang alias diterima jika secara eksplisit bukan kejadian langsung `target`. Definisi dianggap lengkap bila konteksnya benar; menyebut fungsi itu sendiri atau scope luarnya untuk baris definisi diterima. Call melalui function pointer dibedakan antara identitas field dan fungsi yang dipanggil saat runtime. Pernyataan eksplisit salah atau saling bertentangan, lokasi yang salah, atau kejadian yang tidak dilaporkan pada barisnya membuat jawaban tidak lulus. Sikap hati-hati tentang binding yang tidak dibuktikan indeks **tidak** dihitung salah; ini berarti skor inventaris tidak mengukur keberhasilan memastikan identitas.

Legenda: **L** = lulus inventaris menurut aturan di atas; **T** = tidak lulus; urutan kolom mengikuti sesi 1 dan 2. Jawaban yang lulus tidak otomatis bebas dari setiap masalah di luar kriteria ini.

| Kasus | Acuan ringkas dari source: jenis@baris | Reviewer | Tanpa Riltra 1/2 | Dengan Riltra 1/2 |
|---|---|---|---|---|
| c-direct | fungsi@2, call@6 | L | L/L | L/L |
| c-reference | fungsi@2, non-call@6 | L | L/L | T/L |
| c-same_name | field A@2, field B@3, call@5 | L | L/T | L/T |
| cpp-direct | fungsi@2, call@6 | L | L/L | L/L |
| cpp-reference | fungsi@2, non-call@6 | L | L/L | L/L |
| cpp-same_name | fungsi A@2, fungsi B@5, field@8, call@10 | T | T/T | T/T |
| js-direct | fungsi@1, call@2 | L | L/L | L/L |
| py-direct | fungsi@2, call@5 | L | L/L | L/L |
| py-reference | fungsi@2, non-call@5 | T | L/L | L/L |
| py-same_name | metode A@3, metode B@7, call@11 | L | L/L | L/L |
| rs-direct | fungsi@2, call@6 | L | L/L | L/L |
| rs-reference | fungsi@2, non-call@6 | L | L/L | L/L |
| rs-same_name | metode A@4, metode B@5, call@7 | L | L/L | L/L |
| ts-direct | fungsi@2, call@6 | L | L/L | L/L |
| ts-reference | fungsi@2, non-call@6 | L | L/L | L/L |
| ts-same_name | metode A@2, metode B@3, call@5 | T | L/L | L/L |
| tsx-direct | fungsi@1, call@2 | L | L/L | L/L |

Dengan kriteria inventaris ini: reviewer **14/17**, tanpa Riltra **31/34** (91,2%), dengan Riltra **30/34** (88,2%). Persentase dihitung per jawaban, bukan per fakta atau per proyek; selisih satu jawaban tidak membuktikan keunggulan umum. Angka bukan rata-rata akurasi Riltra sebagai engine, bukan hasil semantic provider, dan tidak boleh digeneralisasi ke proyek nyata. Perlakuan terhadap ketidakpastian dan field declaration memengaruhi hasil; aturan ini disusun setelah output tersedia.

## Alasan setiap jawaban tidak lulus

- `c-reference/riltra-1`: non-call dicatat baris 7, seharusnya 6.
- `c-same_name/base-2`: menyatakan field mana yang dipilih tidak pasti sekaligus menyatakan ia menyelesaikan ke field A; dua pernyataan itu bertentangan. Fungsi yang tersimpan pada pointer memang tidak diketahui.
- `c-same_name/riltra-2`: tiga lokasi dilaporkan 1, 2, 4, seharusnya 2, 3, 5; alasan ketidakpastian field juga mengabaikan tipe `struct A *`.
- `cpp-same_name/review-1`: deklarasi field baris 8 diklasifikasikan sebagai referensi non-call.
- `cpp-same_name/base-1`: pemuat call baris 10 ditulis `f`, seharusnya `caller`.
- `cpp-same_name/base-2`: deklarasi field baris 8 diklasifikasikan sebagai referensi non-call.
- `cpp-same_name/riltra-1`: menyebut field `Obj` dalam uraian tetapi tidak memberi nomor baris 8, serta tidak memasukkannya dalam inventaris lokasi.
- `cpp-same_name/riltra-2`: field baris 8 dengan benar dikenali sebagai deklarasi, tetapi juga secara eksplisit disebut referensi non-call.
- `py-reference/review-1`: tabel menambahkan baris `Call reference` pada baris 5 meskipun tidak ada call; kolomnya bertuliskan `None`, sehingga jawabannya setidaknya tidak konsisten.
- `ts-same_name/review-1`: menambahkan selector pada call baris 5 sebagai referensi non-call terpisah.

Pada banyak kasus sederhana, baseline menyatakan identitas pasti berdasarkan source, sementara arm Riltra menyatakan kandidat belum terverifikasi oleh indeks. Keduanya dapat lulus inventaris di atas, tetapi belum memberikan tingkat kepastian yang sama. Pada `rs-same_name`, misalnya, source menunjukkan `obj: &A`, namun kedua jawaban Riltra berhenti pada kandidat ambigu. Ini adalah perbedaan kegunaan yang perlu dinilai terpisah, bukan disembunyikan oleh angka inventaris.

## Batas dan bukti

Prompt kedua arm berbeda: arm Riltra diwajibkan memakai indeks syntax-only dan memeriksa source; baseline memakai pencarian source. Dengan demikian jawaban akhir Riltra tidak membuktikan bahwa semua fakta berasal dari Riltra, dan pembatasan reviewer tidak terbukti secara teknis. Tabel ini menilai jawaban akhir, bukan audit menyeluruh seluruh tool call, kepatuhan instruksi atau atribusi setiap fakta. Pemeriksaan penyelesaian, hash salinan dan keterbatasan model berada di [audit awal](agent-run.md); bukti mentah disimpan lokal dan diabaikan Git.
