# Waktu: epoch, ISO-8601, zona waktu

```bash
dt time                                    # sekarang
dt time 1767323045                         # epoch detik
dt time -z Asia/Jakarta 2026-01-02T10:04:05
dt time -u ms 1000                         # paksa satuan milidetik
dt time -- -86400                          # epoch negatif (pakai "--")
```

Contoh keluaran:

```
UTC:          2026-01-02T03:04:05.000Z
Asia/Jakarta: 2026-01-02T10:04:05.000+07:00
Epoch (s):    1767323045
Epoch (ms):   1767323045000
Relatif:      3 jam lalu
```

## Opsi

- `[nilai]`: tanpa nilai berarti sekarang. Epoch bulat (boleh negatif) atau
  ISO-8601: `2026-01-02`, `2026-01-02T10:04:05`, `2026-01-02 10:04`,
  `2026-01-02T10:04:05.123+07:00`, `2026-01-02T03:04:05Z`.
- `-z, --timezone <IANA>`: zona untuk keluaran dan untuk membaca waktu tanpa
  offset. Default zona sistem. Nama tidak dikenal ditolak.
- `-u, --unit <s|ms>`: satuan epoch, berlaku untuk argumen `[nilai]` dan
  `--now` sekaligus. Default: nilai dengan abs < 1e11 dianggap detik,
  selainnya milidetik. Dengan opsi ini, setiap nilai eksplisit harus berupa
  epoch; nilai ISO atau pemakaian tanpa epoch ditolak.
- `--now <waktu>`: ganti jam sekarang untuk hitungan `Relatif` (ISO atau epoch),
  berguna untuk hasil yang tetap di skrip dan tes. Contoh:
  `dt time -u ms --now 5000 1000` menghasilkan `4 detik lalu`.

## Catatan

- Waktu tanpa offset dibaca di zona `--timezone`, bukan zona mesin. Tanggal saja
  berarti tengah malam di zona itu. Offset eksplisit (`Z`, `+07:00`) selalu menang.
- Pada transisi DST, waktu yang tidak ada digeser maju sepanjang celah waktu:
  `2026-03-08T02:30` di `America/New_York` menjadi `03:30`, demikian juga
  `2026-03-29T02:30` di `Europe/Berlin`. Waktu yang berulang memilih kemunculan
  pertama (instan yang lebih awal), tanpa peringatan.
- `Relatif` memakai satuan terbesar yang dibulatkan ke bawah: detik, menit, jam,
  hari (< 30), bulan (30 hari), tahun (365 hari). Selisih di bawah 1 detik: `sekarang`.
- Epoch negatif diawali `--` agar tidak dibaca sebagai opsi.
- Rentang Date: ±8,64e15 ms (tahun ±275760). Di luar itu ditolak.
- Epoch detik dengan bagian pecahan (`1.5`) tidak didukung.

Exit: 0 sukses, 2 input atau opsi tidak valid (waktu, zona, satuan, rentang).

## Keluaran JSON

Dengan `--json`, stdout berisi satu dokumen JSON; kode keluar tidak berubah.

- `input`: nilai argumen (string) atau `null` bila tidak ada
- `utc`: ISO-8601 UTC
- `zone`: `{ name, iso }` (nama zona dan waktu ISO di zona itu)
- `epochSeconds`, `epochMillis`: angka
- `relative`: teks waktu relatif

[Kembali ke dokumentasi](README.md).
