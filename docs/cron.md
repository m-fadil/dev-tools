# cron

Jelaskan ekspresi cron 5 field dan tampilkan waktu eksekusi berikutnya.

```bash
dt cron "*/15 9-17 * * mon-fri"          # 5 waktu berikutnya dari sekarang
dt cron -n 10 0 3 1 '*' '*'               # 5 argumen terpisah juga boleh
dt cron --utc --from 2025-01-01T00:00:00Z @daily
dt cron --from 2025-03-28T00:00 "30 2 * * *"
```

Keluaran: satu baris penjelasan per field (`menit: 0, 15, 30, 45`,
`hari: Senin, Selasa, ...`, atau `setiap menit`), baris kosong, lalu waktu
berikutnya satu per baris dalam ISO-8601 dengan offset zona waktu sistem
(`2025-03-28T02:30:00+01:00`), atau `Z` dengan `--utc`.

## Opsi

- `-n, --next <n>`: jumlah waktu yang ditampilkan, 1-50 (default 5).
- `--from <waktu>`: hitung setelah waktu ini (eksklusif; detik diabaikan).
  Format `YYYY-MM-DD` atau `YYYY-MM-DDTHH:mm[:ss][Z|+HH:MM]`. Tanpa offset,
  waktu dibaca di zona yang dipakai (sistem, atau UTC dengan `--utc`).
  Tahun `0000`–`0099` dipakai sebagaimana ditulis, bukan dipetakan ke `1900`–`1999`.
  Default: sekarang.
- `--utc`: pakai dan tampilkan UTC, bukan zona waktu sistem.
- `--json`: satu dokumen JSON ke stdout, tanpa penjelasan teks.

## Sintaks

Field: menit (0-59), jam (0-23), tanggal (1-31), bulan (1-12 atau `jan`-`dec`),
hari (0-7 atau `sun`-`sat`; 0 dan 7 = Minggu). Nama tidak membedakan huruf
besar/kecil. Tiap field boleh berisi `*`, nilai `a`, daftar `a,b`, rentang
`a-b`, step `*/n`, `a-b/n`, dan `a/n` (dari `a` sampai maksimum field).
Rentang tidak melingkar: tulis `5-7`, bukan `fri-sun`. Makro: `@yearly`,
`@annually`, `@monthly`, `@weekly`, `@daily`, `@midnight`, `@hourly`
(`@reboot` tidak didukung).

Kalau tanggal DAN hari sama-sama dibatasi (field tidak diawali `*`), waktu
cocok bila salah satunya terpenuhi: `0 0 13 * fri` berarti setiap tanggal 13
ditambah setiap Jumat. Bila salah satunya `*`, hanya field yang lain berlaku.

## Keluaran JSON

```
{ expression, fields: { minute, hour, dayOfMonth, month, dayOfWeek },
  from, timezone: "utc" | "local", next: string[] }
```

`fields` berisi array angka hasil ekspansi (hari 0-6, 0 = Minggu). `from` dan
`next` berformat ISO-8601. Bila pencarian 8 tahun habis, JSON tetap dicetak
dengan `next` yang ditemukan (bisa kosong) dan exit tetap 1.

## Kode exit

0 sukses, 1 tidak ada waktu yang cocok dalam 8 tahun (mis. `0 0 31 2 *`),
2 ekspresi atau opsi salah (pesan menyebut field yang bermasalah).

## Catatan

- Pencarian melompat per bulan, hari, dan jam, bukan per menit, sehingga
  cepat dan pasti berhenti. Batas 8 tahun berlaku untuk tiap waktu terhadap
  waktu sebelumnya (jadi 29 Februari tetap ditemukan melewati tahun 2100).
- DST (zona waktu sistem): jam yang terlewat saat jam maju tidak dijalankan
  (`30 2 * * *` dilewati pada hari itu). Jam yang berulang saat jam mundur
  hanya dijalankan sekali, pada kemunculan pertama, sehingga tidak ada
  duplikat dan hasil selalu naik. Job yang sering (mis. `*/30 * * * *`)
  karenanya tidak muncul pada pengulangan jam itu. Gunakan `--utc` untuk
  menghindari perilaku ini.
- Ini perhitungan dari ekspresi saja; tidak membaca crontab dan tidak
  menjalankan apa pun. Dukungan field detik, tahun, `L`, `W`, `#`, dan `?`
  tidak ada.

[Kembali ke dokumentasi](README.md).
