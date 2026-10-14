# diff

Membandingkan dua file: hash sha256 (streaming), ukuran, lalu diff baris untuk
teks atau offset byte pertama yang berbeda untuk biner.

```bash
dt diff lama.txt baru.txt
dt diff -U 1 lama.txt baru.txt   # 1 baris konteks
dt diff a.bin b.bin
```

Keluaran jika identik:

```
Identik (sha256 ba7816bf...15ad, 3 byte)
```

Jika berbeda: `Berbeda`, ukuran dan sha256 kedua file, lalu header `--- a` / `+++ b`
dan hunk bergaya unified (`@@ -baris,jumlah +baris,jumlah @@`, baris diawali
spasi, `-`, atau `+`). Baris terakhir tanpa newline ditandai
`\ Tidak ada newline di akhir file` dan dianggap berbeda dari baris yang sama
dengan newline.

## Opsi

- `-U, --context <n>`: baris konteks, 0-20 (default 3).
- `--json`: keluaran satu dokumen JSON (lihat Keluaran JSON).
- `-h, --help`: bantuan.

## Exit code

- `0`: identik.
- `1`: berbeda, atau error (misalnya file tidak ada). Seperti `diff`/`cmp`,
  hasil "berbeda" ditulis ke stdout sedangkan pesan error ke stderr, jadi
  keduanya bisa dibedakan.
- `2`: penggunaan salah (bukan tepat dua file, `--context` di luar 0-20).

## Catatan

- File dianggap biner jika ada byte NUL di 8000 byte pertama salah satunya.
  Untuk biner dicetak `Biner, berbeda di byte offset <n>` (offset dari 0).
- Diff baris hanya untuk file teks <= 5 MiB dan jumlah baris a x b <= 25.000.000.
  Di luar batas itu hanya ringkasan yang dicetak, dengan saran memakai
  `git diff --no-index`.
- Baris dipisah oleh `\n`; `\r` dianggap bagian baris. Byte UTF-8 tidak valid
  tetap dibandingkan persis, hanya tampilannya yang diganti karakter pengganti.

## Keluaran JSON

Dengan `--json`, stdout berisi satu dokumen JSON; kode keluar tidak berubah (1 jika berbeda).

- `identical`: boolean
- `a`, `b`: `{ path, size, sha256 }`
- `kind`: `identical`, `text`, `binary`, atau `summary-only` (diff baris dilewati karena terlalu besar)
- `firstDifferingByte`: hanya untuk `binary`
- `hunks`: hanya untuk `text`; tiap elemen `{ aStart, aLines, bStart, bLines, lines: [{ op, text }] }` dengan `op` `' '`, `'-'`, atau `'+'`

[Kembali ke dokumentasi](README.md).
