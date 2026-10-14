# text

Alat teks: statistik, balik, slug, frekuensi kata, dan olah baris.

```bash
dt text stats "Halo dunia. Apa kabar?"
dt text reverse "héllo"
dt text slug "Héllo Wörld!"            # hello-world
cat log.txt | dt text freq --top 5
dt text lines --trim --unique --sort --file daftar.txt
```

Teks dari argumen, `--file`, atau stdin. Satu newline di akhir input diabaikan,
jadi `echo x | dt text stats` sama dengan `dt text stats x`.
Opsi yang tidak berlaku untuk suatu perintah ditolak (exit 2).
"Kata" adalah deret huruf/angka Unicode (boleh dengan `'`, `’`, `-` di tengah).

## Perintah

- `stats`: karakter (code point Unicode), grafem (`Intl.Segmenter`), byte input asli,
  kata, baris, kalimat (berakhir `. ! ? …` diikuti spasi/akhir teks, jadi `3.14`
  utuh), baris terpanjang (dalam grafem, dengan nomor baris), rata-rata panjang kata
  (grafem, 2 desimal). Byte dihitung sebelum decoding UTF-8, bahkan untuk input
  UTF-8 tidak valid; satu newline akhir (`LF` atau `CRLF`) tidak dihitung.
  Penghitungan kalimat memindai teks satu kali, termasuk deret tanda baca panjang.
- `reverse`: balik tiap baris per grafem (emoji dan huruf beraksen tetap utuh).
  `--lines` membalik urutan baris saja.
- `slug`: normalisasi NFD, hilangkan tanda diakritik U+0300–U+036F, lalu
  normalisasi NFC dan huruf kecil. Dakuten kana dan tanda Devanagari dipertahankan.
  Apostrof dibuang; deret karakter selain huruf, tanda gabung, atau angka menjadi
  satu pemisah. `--sep <char>`: pemisah satu karakter selain huruf, angka, atau
  spasi (default `-`). Hasil kosong adalah galat (exit 1).
- `freq`: frekuensi kata, urut jumlah menurun lalu abjad. `--top <n>` (default 10),
  `--case-sensitive` (default: huruf besar/kecil disamakan), `--min-length <n>`
  (default 1, dalam code point).
- `lines`: urutan proses `--trim`, `--unique` (kemunculan pertama), `--sort`
  (urut code point Unicode, bukan locale), `--reverse`, lalu `--numbered` (tidak
  bersama `--json`).

Opsi umum: `-f, --file <path>`, `--json`, `-h, --help`.

## Keluaran JSON

- `stats`: `{ characters, graphemes, bytes, words, lines, sentences, longestLine: { number, length }, averageWordLength }`
  (`longestLine.number` 0 bila tidak ada baris).
- `reverse`: `{ lines: string[] }`.
- `slug`: `{ slug: string }`.
- `freq`: `{ total, unique, top: [{ word, count }] }`; `total` = jumlah kata
  yang dihitung (setelah `--min-length`), `unique` = jumlah kata berbeda.
- `lines`: `{ count, lines: string[] }`.

[Kembali ke dokumentasi](README.md).
