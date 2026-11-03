# Hasil evaluasi agent v2

Tanggal 3 November 2026. [Protokol dan persiapan](agent-v2-run.md) mencatat pin, review oracle, percobaan read-only yang dihentikan, dan perbedaan hak akses. Corpus ini terdiri atas 17 contoh kecil yang sudah dikenal dari v1; ini bukan sampel proyek nyata. Satu reviewer terpisah meninjau oracle sebelum run, tetapi aksesnya ke file lain tidak dibatasi secara teknis.

Seluruh **68/68 sesi** mencapai `turn.completed` dan exit 0, dengan 68 ID sesi serta 68 home berbeda. Hash source tiap salinan cocok sebelum dan sesudah. Dalam log command, 34/34 sesi Riltra mengeluarkan JSON query indeks; tidak ditemukan command baseline yang memuat nama executable Riltra. Pencarian pola command tidak membuktikan semua akses atau asal setiap fakta. Paling sedikit satu baseline membaca file skill di luar fixture; pencarian command tidak menemukan nama oracle/golden, tetapi akses teknis ke file lain tidak dibatasi. Model `gpt-6-luna` diminta pada seluruh sesi dan muncul dalam preflight, tetapi metadata model untuk masing-masing sesi tidak terdapat di event JSON yang diperiksa.

## Penilaian jawaban

Jawaban akhir dibandingkan dengan oracle 40 entri dan source per kasus. Hasil di bawah adalah hitungan **per jawaban lengkap**, 34 jawaban per kondisi, bukan persentase per occurrence atau per proyek. Kolom yang tidak menyebut konvensi dinormalisasi bila interpretasi masuk akal; hitungan `sesuai` di bawah mencakup konvensi yang disimpulkan itu. Bila konvensi atau apakah sebuah selector dihitung terpisah masih tidak jelas, ditandai ambigu. Span seluruh fungsi atau ekspresi yang benar dibedakan dari span identifier dan tidak otomatis disebut salah.

| Dimensi | Tanpa Riltra | Dengan Riltra |
|---|---:|---:|
| Semua lokasi/baris dan jenis sesuai tanpa ambiguitas | 33/34 | 29/34 |
| Span identifier sesuai, termasuk konvensi yang disimpulkan | 20/34 | 23/34 |
| Span identifier jelas salah | 13/34 | 9/34 |
| Span ambigu | 1/34 | 1/34 |
| Span benar tetapi lebih luas dari identifier | 0/34 | 1/34 |
| Lokasi, jenis dan span identifier semuanya sesuai | 20/34 (58,8%) | 20/34 (58,8%) |

Baris terakhir **tidak menilai ketepatan identitas**. Dalam 30/34 jawaban dengan Riltra, setidaknya satu identitas yang dapat dinilai dari source tetap dinyatakan belum terverifikasi; pada empat jawaban lain ketidakpastian receiver memang sesuai oracle. Ini adalah jawaban yang lebih terbatas, bukan otomatis klaim salah. Pada baseline terdapat satu jawaban yang menyatakan identitas field belum pasti meskipun tipe terlihat dan satu jawaban yang terlalu jauh menyatakan fungsi runtime bukan salah satu fungsi namespace. Dua kondisi tidak mendapat instruksi identik: arm Riltra diminta menggunakan indeks dan menjaga kandidat tetap tidak pasti, sedangkan baseline diminta memakai pencarian source. Agent Riltra juga membaca source. Karena itu angka ini tidak membuktikan akurasi engine, tidak menunjukkan bahwa kedua kondisi sama pada identitas, dan tidak berlaku untuk proyek nyata.

Pada arm Riltra, dua jawaban secara eksplisit mengklasifikasikan bagian call sebagai referensi non-call terpisah dan dua jawaban lain mencantumkan selector dan call secara terpisah tanpa klasifikasi non-call yang jelas. Keduanya dibedakan dari kesalahan lokasi; selector dalam call tidak dihitung dua kali dalam oracle.

## Audit setiap sesi

Tabel memperlihatkan empat kode berurutan **lokasi/jenis/span/identitas**. `=` sesuai; `!` salah; `?` ambigu; `~` sesuai jika konvensi kolom yang tidak dinyatakan disimpulkan; `B` span valid tetapi lebih luas daripada identifier; `U` identitas belum dipastikan; `U*` identitas field atau method dapat ditentukan dari tipe receiver di source tetapi jawaban tetap ragu. `=` pada identitas mencakup ketidakpastian yang memang tepat untuk receiver dinamis. Fungsi runtime dalam field function pointer tetap tidak diketahui. Kolom `U` adalah abstain, bukan tuduhan false positive.

| Kasus | Baseline 1 | Baseline 2 | Riltra 1 | Riltra 2 |
|---|---|---|---|---|
| c-direct | =/=/=/= | =/=/=/= | =/=/?/U | =/=/~/U |
| c-reference | =/=/!/= | =/=/~/= | =/=/~/U | !/=/!/U |
| c-same_name | =/=/!/U* | =/=/=/= | =/=/!/U* | =/=/=/U* |
| cpp-direct | =/=/=/= | =/=/!/= | =/=/=/U | =/=/!/U |
| cpp-reference | =/=/~/= | =/=/~/= | =/=/~/U | =/=/=/U |
| cpp-same_name | =/=/!/! | !/=/!/= | =/!/=/U* | =/?/!/U* |
| js-direct | =/=/!/= | =/=/~/= | =/=/=/U | =/=/!/U |
| py-direct | =/=/?/= | =/=/~/= | =/=/=/U | =/=/~/U |
| py-reference | =/=/!/= | =/=/=/= | =/=/!/U | =/=/=/U |
| py-same_name | =/=/!/= | =/=/=/= | =/!/=/= | =/?/=/= |
| rs-direct | =/=/~/= | =/=/=/= | =/=/=/U | =/=/~/U |
| rs-reference | =/=/=/= | =/=/=/= | =/=/~/U | =/=/~/U |
| rs-same_name | =/=/!/= | =/=/!/= | =/=/!/U* | =/=/B/U* |
| ts-direct | =/=/=/= | =/=/!/= | =/=/~/U | =/=/=/U |
| ts-reference | =/=/=/= | =/=/=/= | =/=/=/U | =/=/!/U |
| ts-same_name | =/=/=/= | =/=/=/= | =/=/=/= | =/=/=/= |
| tsx-direct | =/=/!/= | =/=/!/= | =/=/!/U | =/=/~/U |

Contoh temuan: `c-reference/riltra-2` menempatkan referensi pada baris 7 padahal baris 6; `cpp-same_name/base-2` salah pada tiga nomor baris; `tsx-direct/base-1` menempatkan call pada kolom 78 padahal identifier dimulai pada kolom 39. Detail alasan setiap sesi dicatat dalam CSV audit lokal bersama log asli.

Penilaian awal dilakukan operator dengan kriteria di atas. Agent peninjau yang berbeda kemudian menyatakan telah memeriksa seluruh 68 jawaban terhadap source, oracle dan tabel, serta menyoroti lima jawaban dengan identitas terlalu ragu. Penilaian akhir menandai kelima kasus itu dan dua kasus C yang analog sebagai `U*`. Kami mempertahankan pembedaan antara abstain dan klaim salah; reviewer menyebut beberapa abstain itu “salah/terlalu ragu”, sedangkan tabel menunjukkan secara eksplisit bahwa identitas source belum dijawab. Penilaian tetap melibatkan judgment dan sumber review yang sama, bukan bukti kebenaran mutlak.

## Bukti dan keterbatasan

Oracle, review, runner, 68 output, stderr dan status, salinan percobaan awal, serta CSV audit berada secara lokal di `docs/pilots/agent-kebenaran-2026-11-02/raw-local/agent-v2-20261103/`. Sebanyak 204 salinan event, stderr dan status diverifikasi SHA-256 terhadap 68 sesi asli. Raw evidence diabaikan Git, dapat berisi path lokal dan tidak ikut commit; direktori `target/` juga dapat dibersihkan. Penghitungan tidak menggabungkan percobaan read-only yang dihentikan, fixture bukan proyek independen, dan ulangan dua kali tidak cukup untuk generalisasi statistik.
