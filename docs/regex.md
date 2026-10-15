# Data regex

```bash
dt regex '(\d+)-(?<kata>[a-z]+)' "12-ab 7-z"
echo "a.b.c" | dt regex -r '$0!' '\.'
dt regex -f i --first 'error' --file app.log
dt regex --test '^v\d+$' "v12" && echo cocok
dt re --json '\b\w' "halo dunia"
```

Mencari semua kecocokan dengan sintaks crate Rust `regex` pada teks. Teks berasal dari argumen
setelah pattern (digabung dengan spasi), `--file`, atau stdin; stdin dipakai apa
adanya, termasuk newline akhir (jadi `$` cocok sebelum newline hanya dengan flag
`m`, dan `.` tidak melewati newline kecuali flag `s`). Alias: `re`.

Opsi:

- `-f, --flags <flags>`: hanya flag `i m s x`; pencarian global selalu aktif.
  Flag dinormalisasi menjadi huruf unik terurut. Flag lain menghasilkan exit 2:
  `Flag tidak dikenal: <c>. Pilihan: imsx`.
- `-r, --replace <template>`: cetak juga hasil penggantian dengan sintaks Rust
  (`$1`, `${nama}`, `$0`, `$$`). Gunakan `${nama}` untuk memisahkan nama grup
  dari teks sesudahnya. Semua kecocokan diganti, atau hanya yang pertama bila `--first`.
- `--first`: berhenti di kecocokan pertama.
- `--test`: tanpa keluaran; exit 0 bila ada kecocokan, 1 bila tidak. Tidak bisa
  digabung dengan `--json` atau `--replace`.
- `--json`: keluaran JSON (lihat di bawah).
- `--file <path>`: baca teks dari berkas.

Keluaran per kecocokan:

```
#1 index=0 "12-ab"
  $1 "12"
  $2 "ab"
  <kata> "ab"
```

`index` adalah offset byte UTF-8, bukan indeks UTF-16. Isi kecocokan dan
grup ditulis sebagai string JSON (karakter khusus di-escape); grup yang tidak ikut
cocok ditampilkan `-`. Dengan `--replace`, hasil dicetak setelahnya di bawah
`Hasil ganti:`. Bila tidak ada kecocokan, pesan `Tidak ada kecocokan.` ditulis ke
stderr dan exit tetap 0 (kecuali `--test`).

Kecocokan kosong (mis. `x*`) tidak membuat loop tak berujung; pencarian berikutnya
maju ke batas karakter UTF-8 berikutnya.

## Keluaran JSON

Satu dokumen JSON:

- `pattern`: string, pattern apa adanya.
- `flags`: string, flag pengguna dinormalisasi menjadi huruf unik terurut (`imsx`).
- `matches`: array objek `{ index, match, groups, named }`; `groups` adalah array
  string atau `null` (grup tak ikut cocok) berurutan mulai `$1`, `named` adalah
  objek nama grup → string atau `null` (`{}` bila tidak ada grup bernama).
- `replaced`: string hasil penggantian; hanya ada bila `--replace` dipakai.

## Sintaks dan keamanan

Mesin `regex` tidak memakai backtracking dan tidak mendukung lookaround atau
backreference. Pattern yang tidak didukung menghasilkan exit 2 dengan pesan
`Regex tidak valid: <galat regex>`. Pencarian tunggal memiliki batas waktu linear
terhadap panjang input untuk ukuran pattern tetap; enumerasi semua kecocokan
dapat membutuhkan kerja tambahan.

Sejak v1.0.0, `--timeout`, worker, dan batas heap worker dihapus. Tidak ada lagi
error timeout/batas memori worker. Template JavaScript `$&` dan `$<nama>` tidak
didukung; gunakan `$0` dan `${nama}`.

Exit: 0 sukses, 1 (`--test`) tidak cocok atau gagal membaca input, 2 pattern atau
penggunaan salah. Pattern yang diawali `-` ditulis setelah `--`.

[Kembali ke dokumentasi](README.md).
