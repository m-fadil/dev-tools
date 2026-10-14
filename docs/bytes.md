# Utilitas bytes

```bash
dt bytes 1536
dt bytes 1.5GB
dt bytes "2 GiB" --to MB
dt bytes 10kb -p 0 --json
```

Mengonversi ukuran data antar satuan dengan aritmetika eksak (BigInt/rasional,
tanpa galat float: `0.1 KB` tepat 100 byte). Ukuran berupa angka desimal plus
satuan opsional (boleh dipisah spasi); angka tanpa satuan berarti byte.
Negatif ditolak.

Satuan (huruf besar/kecil tidak dibedakan):

- Desimal (basis 1000): `KB`/`kB`, `MB`, `GB`, `TB`, `PB`.
- Biner (basis 1024): `KiB`, `MiB`, `GiB`, `TiB`, `PiB`.
- `B` untuk byte. Jadi `kb` = 1000 byte sedangkan `kib` = 1024 byte.

Keluaran default: jumlah byte, tabel seluruh satuan desimal dan biner, lalu baris
`Terbaca:` berisi satuan terbesar dengan nilai >= 1 untuk kedua sistem
(mis. `Terbaca: 1.5 GB | 1.4 GiB`; di bawah 1 KB tampil dalam `B`).

Jika pembulatan sesuai `--precision` mencapai basis satuan (1000 atau 1024),
`Terbaca:` naik ke satuan berikutnya: `999999` byte menjadi `1 MB` dan `1048575`
byte menjadi `1 MiB` (presisi default). Satuan tidak naik melebihi `PB`/`PiB`.
Tabel dan `--to` tetap memakai satuan yang diminta; aturan ini juga berlaku
pada `readable` dalam JSON.

Opsi:

- `-t, --to <satuan>`: cetak hanya nilai konversi ke satuan itu (angka saja,
  mudah dipakai di skrip). `B` juga diterima.
- `-p, --precision <n>`: jumlah angka desimal 0-10 (default 2); pembulatan
  setengah ke atas, nol di ujung dibuang. Nilai negatif setelah opsi ini
  (mis. `bytes 1 -p -1`) ditolak sebagai presisi tidak valid, bukan ukuran negatif.
- `--json`: keluaran JSON.

Jika ukuran bukan byte bulat (mis. `1.5` atau `0.5B`), hasil dibulatkan setengah
ke atas, ditandai `(dibulatkan)`, dan peringatan dicetak ke stderr. Seluruh
konversi memakai jumlah byte yang sudah dibulatkan itu.

Exit: 0 sukses, 2 ukuran/satuan/opsi tidak valid.

## Keluaran JSON

Semua angka berupa string agar tidak kehilangan ketelitian.

- `input`: teks ukuran seperti diberikan.
- `bytes`: jumlah byte (bulat).
- `rounded`: `true` bila dibulatkan.
- `precision`: angka desimal yang dipakai.
- `decimal`: objek `KB`, `MB`, `GB`, `TB`, `PB` berisi nilai.
- `binary`: objek `KiB`, `MiB`, `GiB`, `TiB`, `PiB` berisi nilai.
- `readable`: `{ decimal, binary }`, mis. `"1.5 GB"`.

Dengan `--to`: `input`, `bytes`, `rounded`, `unit` (nama baku, mis. `GiB`), `value`.

[Kembali ke dokumentasi](README.md).
