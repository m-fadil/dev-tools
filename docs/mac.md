# mac

Parse dan konversi alamat MAC (EUI-48), atau buat alamat acak.

```bash
dt mac 00-1A-2B-3C-4D-5E                  # semua bentuk + flag
dt mac -f dot 00:1a:2b:3c:4d:5e           # 001a.2b3c.4d5e
dt mac --upper -f hyphen aabbccddeeff
echo 'aabb.ccdd.eeff' | dt mac -f colon   # tanpa argumen: baca stdin
dt mac --random --json
```

Bentuk input yang diterima: colon `aa:bb:cc:dd:ee:ff`, hyphen
`aa-bb-cc-dd-ee-ff`, dot Cisco `aabb.ccdd.eeff`, dan plain `aabbccddeeff`
(tanpa campuran pemisah; huruf besar/kecil bebas). Spasi/baris baru di awal dan
akhir diabaikan, baik dari argumen maupun stdin. Alamat tidak valid: pesan
galat (menampilkan teks yang sudah di-trim) dan exit 2.

Tanpa `--format`, dicetak keempat bentuk (colon, hyphen, dot, plain), tipe
`unicast`/`multicast` (bit I/G: bit terendah oktet pertama), dan
`universally`/`locally administered` (bit U/L: bit kedua terendah).

## Opsi

- `-f, --format <jenis>`: cetak satu bentuk saja: `colon`, `hyphen`, `dot`, `plain`.
- `--upper`: huruf kapital (default huruf kecil).
- `--random`: buat alamat acak (CSPRNG) yang unicast dan locally administered;
  tidak boleh disertai argumen alamat. Bisa digabung dengan `--format`/`--json`.
- `--json`: keluaran JSON.

## Kode exit

- `0` sukses; `2` alamat, opsi, atau argumen salah.

## Keluaran JSON

`address` (bentuk terpilih lewat `--format`, default colon), `formats` (objek
`colon`, `hyphen`, `dot`, `plain`), `multicast` (boolean), `locallyAdministered`
(boolean). `--upper` berlaku untuk semua string.

[Kembali ke dokumentasi](README.md).
