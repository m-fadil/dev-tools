# Utilitas hash, encrypt, encode, decode, random

Hash, encrypt, encode, dan decode membaca input dari argumen teks,
`--file <path>`, atau stdin (dipakai apa adanya, termasuk newline akhir). Tanpa ketiganya
di terminal, command menampilkan prompt. Prompt ditulis ke stderr, jadi stdout
aman dipipe. `random` tidak membaca teks/file/stdin; ia menghasilkan data baru.
Exit: 0 sukses, 1 gagal (input rusak, password salah), 2 penggunaan salah.

## hash

```bash
dt hash "halo"                       # sha256, hex
dt hash -a sha512 -e base64 -f file  # algoritma dan encoding lain
dt hash -k kunci "pesan"             # HMAC
```

Algoritma: `md5`, `sha1`, `sha224`, `sha256`, `sha384`, `sha512`, `sha3-256`,
`sha3-512`, `blake2b512`, `crc32`. Encoding: `hex`, `base64`, `base64url`. `md5` dan `sha1`
hanya untuk checksum/kompatibilitas, bukan keamanan.

`crc32` (`dt hash -a crc32 123456789` → `cbf43926`) adalah checksum 32-bit
untuk deteksi kesalahan, bukan hash kriptografis. Hex 8 digit berpadding nol;
`base64`/`base64url` meng-encode 4 byte big-endian. `--key` dengan `crc32` ditolak
(exit 2) karena bukan HMAC, sebelum membaca input atau menampilkan prompt input.

### Keluaran JSON

`--json` mencetak satu dokumen: `{algorithm, encoding, hmac, digest}` (`hmac`
boolean, true bila `--key` dipakai).

## encrypt

```bash
dt encrypt -p rahasia "halo" > data.txt
dt encrypt -d -p rahasia -f data.txt
DEV_TOOL_PASSWORD=rahasia dt encrypt -f .env > .env.enc
```

AES-256-GCM; kunci 32 byte diturunkan dari password dengan scrypt
(`N=16384`, `r=8`, `p=1`) dan salt acak per operasi. Token versi 1 berupa
base64url dengan susunan: versi (1 byte), salt (16 byte), IV (12 byte), tag
autentikasi (16 byte), lalu ciphertext. Byte versi diikat sebagai GCM AAD;
versi mengidentifikasi format termasuk parameter scrypt. Data yang diubah,
token terpotong, versi tak dikenal, atau password salah ditolak (exit 1).
Token dari build awal yang belum mengikat versi sebagai AAD tidak kompatibel.
Tanpa `--password` atau `DEV_TOOL_PASSWORD`, password diminta lewat prompt
tersembunyi. `--password ''` selalu ditolak (exit 2), tidak memakai nilai env
atau prompt sebagai pengganti. `--password` terlihat di daftar proses dan
history shell.

## encode dan decode

```bash
dt encode "halo"                  # base64: aGFsbw==
dt encode -t url "a b&c"          # a%20b%26c
dt decode -t hex 68616c6f
dt decode -f data.b64 > data.bin  # biner ditulis apa adanya
```

Jenis (`-t`): `base64` (default), `base64url`, `hex`, `url`. Decode bersifat
ketat: input yang bukan jenis tersebut menghasilkan error, bukan data terpotong.
Whitespace di mana pun dalam input `base64`, `base64url`, dan `hex` diabaikan,
termasuk baris terbungkus 76 kolom atau isi body PEM (tanpa header/footer PEM).
Base64 kedua alfabet menerima input tanpa padding; bila ada padding `=`,
panjang total setelah whitespace dibuang harus kelipatan 4 dan jumlah padding
harus tepat (`YQ==` valid; `YQ=` dan `YWJj=` tidak). Bit sisa harus nol
(`YR==` dan `YR` tidak valid).

URL encoding bekerja pada byte mentah, bukan konversi UTF-8, sehingga data
biner bisa round-trip. `A-Z`, `a-z`, `0-9`, dan `-_.!~*'()` dibiarkan literal;
byte lain menjadi `%XX` (hex huruf besar). Decode menerima hex huruf besar
atau kecil, mewajibkan dua digit hex setelah setiap `%`, dan mempertahankan
`+` sebagai tanda plus (bukan spasi).

### Keluaran JSON

`encode --json` mencetak `{type, output}`. `decode` dan `encrypt` tidak punya `--json`.

## random

`dt random` atau `dt random -h` menampilkan pilihan subcommand `uuid` dan `token`.
Opsi diberikan setelah subcommand, bukan setelah `random` saja. Command lama
`dt uuid` dan flag `--token` tidak lagi tersedia.

### UUID

```bash
dt random uuid                    # satu UUID v4
dt random uuid -c 3               # tiga UUID v4
dt random uuid -v 7 -c 3          # tiga UUID v7 (urut waktu)
dt random uuid -v 1               # UUID v1
dt random uuid -h                 # bantuan UUID
```

UUID berformat baku 36 karakter; panjang dan formatnya tidak bisa diubah.
Opsi UUID: `-v/--version` (`1`, `4`, `7`, default `4`), `-c/--count` (1-1000,
default `1`), `--json`, dan `-h/--help`. Tidak menerima argumen teks atau opsi
token (`--length`, `--bytes`, `--bits`, `--format`).

UUID v4 berisi 122 bit acak dan 6 bit penanda versi/varian. UUID berfungsi
sebagai identifier, bukan rahasia. Untuk rahasia pakai `dt random token`.

- **v7** (RFC 9562): 48 bit timestamp unix milidetik, nibble versi `7`, varian `10`,
  12 bit `rand_a` dipakai sebagai counter (diawali nilai acak), 62 bit acak.
  Dalam satu eksekusi hasilnya naik ketat, juga untuk `--count` dalam milidetik
  yang sama maupun bila jam mundur; bila counter meluap, timestamp dimajukan
  1 ms. Urutan leksikografis sama dengan urutan waktu.
- **v1**: timestamp 60 bit dalam satuan 100 ns sejak 1582-10-15, clock sequence
  14 bit acak, identitas simpul 48 bit acak dengan bit multicast diset (RFC 9562 6.10);
  alamat MAC asli tidak pernah dibaca. Timestamp naik ketat dalam satu eksekusi,
  jadi nilai tidak pernah kembar. Identitas simpul dan clock sequence tetap per eksekusi.
  v1 tidak terurut secara leksikografis.

### Token

```bash
dt random token                         # 32 byte, 64 karakter hex
dt random token --bits 256 -f base64url
dt random token -l 24 -f alnum            # tepat 24 karakter A-Za-z0-9
dt random token -c 3 --json
dt random token -h                       # bantuan token
```

Token memakai CSPRNG sistem operasi; tidak perlu flag `--token`. Opsi token:
`-l/--length`, `-b/--bytes`, `--bits`, `-f/--format`, `-c/--count`,
`--json`, dan `-h/--help`. Tidak menerima argumen teks atau `--version`.

Ukuran token: pilih salah satu dari `--length` (karakter, 1-4096), `--bytes`
(1-1024), atau `--bits` (kelipatan 8, 8-8192). Default 32 byte, atau 32 karakter
untuk `alnum`. Format: `hex` (2 karakter/byte), `base64`, `base64url` (~1,33
karakter/byte), dan `alnum` (A-Za-z0-9, hanya dengan `--length`). Dengan
`--length`, hasil dipotong dan base64 tidak memakai padding `=`; entropi tetap
4 bit/karakter (hex) atau 6 bit/karakter (base64, alnum ~5,95). `--count` 1-1000.

### Keluaran JSON

`--json` mencetak `{type, version?, format?, values}`: `type` `"uuid"` atau
`"token"`; `version` (angka 1/4/7) hanya untuk uuid; `format` hanya untuk token;
`values` array string sesuai `--count`.

[Kembali ke dokumentasi](README.md).
