# Data regex

```bash
dt regex '(\d+)-(?<kata>[a-z]+)' "12-ab 7-z"
echo "a.b.c" | dt regex -r '$&!' '\.'
dt regex -f i --first 'error' --file app.log
dt regex --test '^v\d+$' "v12" && echo cocok
dt re --json '\b\w' "halo dunia"
```

Mencari semua kecocokan regex JavaScript pada teks. Teks berasal dari argumen
setelah pattern (digabung dengan spasi), `--file`, atau stdin; stdin dipakai apa
adanya, termasuk newline akhir (jadi `$` cocok sebelum newline hanya dengan flag
`m`, dan `.` tidak melewati newline kecuali flag `s`). Alias: `re`.

Opsi:

- `-f, --flags <flags>`: flag `d i m s u v y`. Flag `g` selalu dipakai secara
  internal untuk mendaftar semua kecocokan. Flag tak dikenal, duplikat, atau
  kombinasi yang ditolak mesin (mis. `uv`) menjadi kesalahan penggunaan.
- `-r, --replace <template>`: cetak juga hasil penggantian dengan sintaks
  JavaScript (`$1`, `$<nama>`, `$&`, `` $` ``, `$'`, `$$`). Semua kecocokan diganti,
  atau hanya yang pertama bila `--first`.
- `--first`: berhenti di kecocokan pertama.
- `--test`: tanpa keluaran; exit 0 bila ada kecocokan, 1 bila tidak. Tidak bisa
  digabung dengan `--json` atau `--replace`.
- `--timeout <ms>`: batas waktu pencocokan, 1-60000 (default 2000).
- `--json`: keluaran JSON (lihat di bawah).
- `--file <path>`: baca teks dari berkas.

Keluaran per kecocokan:

```
#1 index=0 "12-ab"
  $1 "12"
  $2 "ab"
  <kata> "ab"
```

`index` adalah posisi dalam unit UTF-16 (seperti `RegExp`). Isi kecocokan dan
grup ditulis sebagai string JSON (karakter khusus di-escape); grup yang tidak ikut
cocok ditampilkan `-`. Dengan `--replace`, hasil dicetak setelahnya di bawah
`Hasil ganti:`. Bila tidak ada kecocokan, pesan `Tidak ada kecocokan.` ditulis ke
stderr dan exit tetap 0 (kecuali `--test`).

Kecocokan kosong (mis. `x*`) tidak membuat loop tak berujung: posisi maju satu
karakter (satu code point dengan flag `u`/`v`).

## Keluaran JSON

Satu dokumen JSON:

- `pattern`: string, pattern apa adanya.
- `flags`: string, flag pengguna dinormalisasi mesin (tanpa `g` yang ditambahkan internal).
- `matches`: array objek `{ index, match, groups, named }`; `groups` adalah array
  string atau `null` (grup tak ikut cocok) berurutan mulai `$1`, `named` adalah
  objek nama grup → string atau `null` (`{}` bila tidak ada grup bernama).
- `replaced`: string hasil penggantian; hanya ada bila `--replace` dipakai.

## Timeout dan keamanan

Pencocokan berjalan di `worker_threads` Worker yang dihentikan paksa saat
`--timeout` terlampaui, sehingga pola dengan catastrophic backtracking (mis.
`^(a+)+$` pada `aaaa…!`) tidak membuat proses menggantung. Hasilnya error exit 1:
`regex timeout (kemungkinan catastrophic backtracking)`. Waktu start worker ikut
dihitung. Old-generation heap worker dibatasi 256 MB; melebihinya (mis. `--replace`
yang menggandakan keluaran) menghasilkan error exit 1: `regex melebihi batas memori`.

Exit: 0 sukses, 1 timeout/batas memori atau (`--test`) tidak cocok, 2 pattern atau penggunaan
salah (pesan berisi teks galat mesin regex). Pattern yang diawali `-` ditulis
setelah `--`.

[Kembali ke dokumentasi](README.md).
