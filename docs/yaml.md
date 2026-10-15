# yaml

```bash
dt yaml -f config.yml                         # validasi + format ulang
dt yaml --validate -f config.yml              # valid / invalid: baris:kolom pesan
dt yaml --to json -f config.yml               # YAML -> JSON
dt yaml --from json -f data.json              # JSON -> YAML
dt yaml -q production.host -r -f config.yml   # ambil satu nilai
dt yaml --all --to json -f manifests.yml      # semua dokumen sebagai array JSON
```

Input dari argumen teks, `--file <path>`, atau stdin; tanpa ketiganya di terminal muncul prompt (ke stderr).
Sama seperti [`dt json`](json.md), dan sintaks `--query` juga sama.

Contoh (`app.yml`):

```yaml
# konfigurasi
defaults: &defaults
  adapter:  postgres
  pool: 5
production:
  <<: *defaults
  host: db.example.com
ids: [9007199254740993, 18446744073709551615]
```

```console
$ dt yaml -f app.yml
defaults:
  adapter: postgres
  pool: 5
production:
  host: db.example.com
  adapter: postgres
  pool: 5
ids:
- 9007199254740993
- 18446744073709551615

$ dt yaml --to json -m -f app.yml
{"defaults":{"adapter":"postgres","pool":5},"production":{"host":"db.example.com","adapter":"postgres","pool":5},"ids":[9007199254740993,18446744073709551615]}

$ dt yaml -q production.host -r -f app.yml
db.example.com
```

## Opsi

- Tanpa `--to`/`--from`: validasi lalu cetak ulang sebagai YAML yang dinormalisasi. **Komentar tidak dipertahankan**,
  begitu pula gaya penulisan (indentasi, tanda kutip, flow `[a, b]` jadi blok).
- `--validate`: hanya validasi. Cetak `valid` (exit 0), atau `invalid: baris:kolom pesan` ke stderr (exit 1).
  Semua dokumen dalam aliran diperiksa. Tidak bisa digabung dengan opsi lain selain input.
- `--to json`: keluaran JSON dengan indentasi 2 spasi. `-m`, `--minify` untuk satu baris.
- `--from json`: masukan adalah JSON (pemindai ketat yang sama dengan `dt json`, nesting maks 2000), keluaran YAML.
  `--to` dan `--from` tidak bisa bersamaan.
- `--all`: proses semua dokumen (dipisah `---`). Keluaran YAML dipisah baris `---`; dengan `--to json` jadi satu array
  (juga untuk satu dokumen). Tanpa `--all` hanya dokumen pertama yang dibaca dan dicetak. Tidak bisa dengan `--from`.
- `-s`, `--sort-keys`: urutkan key mapping secara rekursif menurut teks key (urutan array tetap).
  Tanpa opsi ini urutan key input dipertahankan.
- `-q`, `--query <path>`: ambil sebagian nilai: `a.b[0].c`, `$.a`, `["key.dengan.titik"]`; `$` atau `.` berarti
  seluruh dokumen (aturan indeks sama dengan `dt json`). Query diterapkan sebelum keluaran; dengan `--all`
  diterapkan pada tiap dokumen. Hasil dicetak dalam format keluaran (YAML, atau JSON dengan `--to json`).
  Key angka (`1: x`) cocok dengan `-q 1`.
- `-r`, `--raw`: dengan `--query`, nilai string dicetak tanpa tanda kutip. Tidak bisa dengan `--all`.
- `-f`, `--file <path>`: baca dari file. BOM di awal diabaikan. Tidak bisa bersama teks posisional.

## Exit code

0 sukses; 1 YAML/JSON tidak valid, `path tidak ditemukan: <path>` (dengan `--all` ditambah `(dokumen N)`), nilai tak bisa
dikonversi, file tak terbaca; 2 penggunaan salah (opsi bentrok, nilai `--to`/`--from` tak dikenal, path query rusak).

## Catatan

- **Batasan merge key:** key `<<` selalu dianggap merge key, juga bila ditulis dengan tanda kutip (`"<<"`), karena
  pustaka YAML tidak membedakannya. Data dengan key `<<` literal tidak bisa diformat ulang atau dikonversi dengan benar.
- **Anchor, alias, dan merge key** (`&a`, `*a`, `<<: *a`) selalu diurai menjadi nilainya, baik saat format ulang maupun
  konversi. Merge key: key tertulis menang atas hasil merge, lalu sumber yang lebih awal; key hasil merge ditaruh
  setelah key milik mapping itu. Alias tidak ditulis ulang.
- **Bilangan bulat** sampai 64-bit (`-9223372036854775808` .. `18446744073709551615`) dipertahankan persis di kedua
  arah, juga di atas 2^53. Batas pustaka YAML: bilangan bulat yang lebih lebar **ditolak** (exit 1), bukan dibulatkan:
  `invalid: 1:5 id: bilangan bulat di luar jangkauan 64-bit (i64/u64): 123456789012345678901234567890`. Sebaliknya,
  JSON dengan bilangan bulat di luar 64-bit ditolak saat `--from json` (`bilangan bulat di a.id di luar jangkauan`).
  Angka yang tertulis desimal/eksponen (`1.5`, `1e3`) adalah float 64-bit dan bisa kehilangan presisi.
- Konversi ke JSON: key bukan string (`1`, `true`, `null`) menjadi string (`"1"`); tag (`!x`) dibuang; key berupa
  urutan/mapping, `.nan`/`.inf`, dan key ganda hasil konversi (`1:` dan `"1":`) ditolak dengan menyebut path-nya
  (mis. `.nan di a[1] tak bisa jadi JSON`). Format ulang YAML -> YAML tidak punya batasan ini.
- Key ganda dalam satu mapping YAML dianggap tidak valid. Kedalaman nesting YAML dibatasi pustaka (128 level):
  `invalid: ... recursion limit exceeded`.
- Input kosong atau hanya komentar adalah satu dokumen `null`. Input bukan UTF-8 yang valid ditolak.
- Posisi error dari pemindai libyaml; kolom dihitung per karakter, mulai dari 1.

[Kembali ke dokumentasi](README.md).
