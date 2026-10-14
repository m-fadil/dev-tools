# Bandingkan file .env

```bash
dt env .env .env.example              # nilai disamarkan
dt env -s .env .env.production        # tampilkan nilai: KEY: a -> b
dt env -k .env .env.example           # bandingkan nama kunci saja
```

Memerlukan tepat dua file. Opsi: `-s/--show-values` menampilkan nilai,
`-k/--keys-only` mengabaikan perbedaan nilai. Contoh keluaran:

```text
Hanya di a:
  DEBUG
Hanya di b:
  SENTRY_DSN
Beda nilai:
  DB_HOST
3 sama, 1 beda, 1 hanya a, 1 hanya b
```

Bagian yang kosong tidak dicetak; baris ringkasan selalu muncul. Kunci diurutkan.
Nilai tidak pernah dicetak tanpa `--show-values`, termasuk di pesan error. Dengan
`--show-values`, nilai muncul di stdout (newline ditampilkan sebagai `\n`), jadi
hati-hati dengan log CI dan history terminal.

## Format yang dibaca

- `KEY=VALUE`, boleh diawali `export `. Baris kosong dan baris `#` diabaikan.
- Nilai `'...'` dibaca apa adanya; nilai `"..."` mendukung `\n`, `\"`, dan `\\`.
  Nilai berkutip boleh memuat `#`, dan boleh diikuti komentar ` # ...`.
- Nilai tanpa kutip di-trim dan komentar ` #...` di akhir dibuang (`COLOR=#fff`
  tetap nilai `#fff`; `#` harus didahului spasi untuk menjadi komentar).
- Tidak ada interpolasi variabel: `${VAR}` dan `$VAR` dibaca sebagai teks biasa.
- Kunci duplikat: nilai terakhir dipakai dan peringatan ditulis ke stderr.
- Baris tidak valid (tanpa `=`, nama kunci salah, kutip tidak tertutup, teks
  setelah kutip penutup) dilaporkan ke stderr sebagai `a:<baris>: baris tidak valid`
  dan dihitung. File dibaca sebagai UTF-8; BOM dan CRLF diterima.

## Exit code

Seperti `diff`: 0 tidak ada perbedaan, 1 ada perbedaan (kunci atau nilai) atau ada
baris tidak valid, atau file tidak bisa dibaca; 2 penggunaan salah (jumlah file
bukan dua, opsi tidak dikenal).

## Keluaran JSON

Dengan `--json`, stdout berisi satu dokumen JSON; aturan kode keluar tidak berubah.

- `onlyA`, `onlyB`: daftar kunci
- `different`: `[{ key, a?, b? }]`; `a`/`b` (nilai) hanya ada dengan `--show-values`
- `same`: jumlah kunci yang sama
- `invalidLines`: `[{ file, line }]` (path file dan nomor baris)

[Kembali ke dokumentasi](README.md).
