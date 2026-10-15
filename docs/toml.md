# toml

```bash
dt toml -f Cargo.toml                       # validasi + format ulang
dt toml --validate -f Cargo.toml            # valid / invalid: baris:kolom pesan
dt toml --to json -f Cargo.toml             # TOML -> JSON
dt toml --from json -f data.json            # JSON -> TOML
dt toml -q package.version -r -f Cargo.toml # ambil satu nilai
```

Input dari argumen teks, `--file <path>`, atau stdin; tanpa ketiganya di terminal muncul prompt (ke stderr).
Opsi dan sintaks `--query` sama dengan [`dt json`](json.md) dan [`dt yaml`](yaml.md) (tanpa `--all`: TOML hanya satu dokumen).

Contoh (`Cargo2.toml`):

```toml
# paket
[package]
name   = "demo"
version = "1.0.0"
released = 2024-03-01T10:00:00Z

[dependencies]
serde = { version = "1", features = ["derive"] }

[[bin]]
name = "dt"
path = "src/main.rs"
```

```console
$ dt toml -f Cargo2.toml
[package]
name = "demo"
version = "1.0.0"
released = 2024-03-01T10:00:00Z

[dependencies.serde]
version = "1"
features = ["derive"]

[[bin]]
name = "dt"
path = "src/main.rs"

$ dt toml -q package.version -r -f Cargo2.toml
1.0.0

$ dt toml -q dependencies.serde -f Cargo2.toml
version = "1"
features = ["derive"]

$ dt toml -q package.released --to json -f Cargo2.toml
"2024-03-01T10:00:00Z"

$ echo '{"name":"app","ports":[80,443],"limits":{"cpu":1.5},"mirrors":[{"url":"a"},{"url":"b"}]}' | dt toml --from json
name = "app"
ports = [80, 443]

[limits]
cpu = 1.5

[[mirrors]]
url = "a"

[[mirrors]]
url = "b"
```

## Opsi

- Tanpa `--to`/`--from`: validasi lalu cetak ulang sebagai TOML yang dinormalisasi. **Komentar tidak dipertahankan**;
  tabel inline (`{ a = 1 }`) ditulis sebagai tabel biasa dan nilai sederhana selalu ditulis sebelum sub-tabel
  (aturan TOML), sehingga posisi key bisa bergeser demi itu. Selain itu urutan key input dipertahankan.
- `--validate`: hanya validasi. Cetak `valid` (exit 0), atau `invalid: baris:kolom pesan` ke stderr (exit 1).
  Tidak bisa digabung dengan opsi lain selain input.
- `--to json`: keluaran JSON dengan indentasi 2 spasi. `-m`, `--minify` untuk satu baris. Datetime TOML
  (offset, lokal, tanggal, waktu) menjadi string apa adanya (RFC 3339), mis. `"2024-03-01T10:00:00Z"`.
- `--from json`: masukan adalah JSON (pemindai ketat yang sama dengan `dt json`, nesting maks 2000), keluaran TOML. Parser TOML sendiri membatasi kedalaman sekitar 64-100 tingkat, jadi keluaran yang sangat bersarang bisa gagal dibaca ulang (`invalid: recursion limit`).
  Objek di tingkat atas wajib; `null` tidak ada di TOML. String JSON yang berbentuk tanggal tetap string.
  `--to` dan `--from` tidak bisa bersamaan.
- `-s`, `--sort-keys`: urutkan key tabel secara rekursif (urutan array tetap).
- `-q`, `--query <path>`: ambil sebagian nilai: `a.b[0].c`, `$.a`, `["key.dengan.titik"]`; `$` atau `.` berarti seluruh
  dokumen. Tabel hasil query dicetak sebagai dokumen TOML; nilai lain (string, angka, array, datetime) dicetak sebagai
  literal TOML (`"TOML"`, `[1, 2]`, `1979-05-27`). Dengan `--to json` hasilnya JSON. Dengan `--from json`, query
  diterapkan pada JSON sebelum konversi, jadi `null` di luar bagian yang diambil tidak mengganggu.
- `-r`, `--raw`: dengan `--query`, nilai string dicetak tanpa tanda kutip.
- `-f`, `--file <path>`: baca dari file. BOM di awal diabaikan. Tidak bisa bersama teks posisional.

## Exit code

0 sukses; 1 TOML/JSON tidak valid, `path tidak ditemukan: <path>`, nilai tak bisa dikonversi, file tak terbaca;
2 penggunaan salah (opsi bentrok, nilai `--to`/`--from` tak dikenal, path query rusak).

## Error konversi

```console
$ echo '{"name":"app","x":null}' | dt toml --from json
TOML tidak punya null: nilai null di x

$ echo '[1,2]' | dt toml --from json
TOML harus berupa tabel (objek) di tingkat atas, bukan array

$ echo 'n = 99999999999999999999' | dt toml
invalid: 1:5 bilangan bulat di luar jangkauan 64-bit (i64): 99999999999999999999
```

Path memakai sintaks query (`a.b[0]`, `a["key.titik"]`).

## Catatan

- **Bilangan bulat** TOML adalah i64 (`-9223372036854775808` .. `9223372036854775807`): semuanya, juga di atas 2^53,
  dipertahankan persis di kedua arah. Di luar itu **ditolak** (exit 1), bukan dibulatkan; berlaku untuk TOML masukan
  dan untuk JSON dengan `--from json` (`bilangan bulat di a.id di luar jangkauan TOML (i64): ...`).
  Angka desimal/eksponen adalah float 64-bit.
- `nan`/`inf` valid di TOML (dan lolos format ulang) tetapi tak bisa jadi JSON: `NaN di a tak bisa jadi JSON`.
- Key duplikat, tabel yang didefinisikan dua kali, dan string tanpa kutip adalah error dengan posisinya.
  Kolom dihitung per karakter, mulai dari 1.
- Input kosong adalah tabel kosong: format ulang tidak mencetak apa-apa, `--to json` mencetak `{}`.
  Input bukan UTF-8 yang valid ditolak.

[Kembali ke dokumentasi](README.md).
