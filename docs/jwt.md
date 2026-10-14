# jwt

Decode JWT, verifikasi signature HMAC bila diberi kunci, dan buat (tanda tangani) token HS256/384/512.

```bash
dt jwt eyJhbGciOiJIUzI1NiIs...        # decode saja
dt jwt "Bearer eyJhbGciOi..."          # awalan Bearer dibuang
pbpaste | dt jwt -k rahasia            # verifikasi HS256/384/512
dt jwt --now 2026-01-01T00:00:00Z -f token.txt
dt jwt -s -k rahasia --exp 1h --iat '{"sub":"42"}'   # buat token
```

Token dibaca dari argumen, `--file <path>`, atau stdin. Spasi di tepi dan
awalan `Bearer ` dibuang. Tanpa ketiganya di terminal, command menampilkan
prompt di stderr.

Output: `Header:` dan `Payload:` sebagai JSON (indentasi 2 spasi), lalu untuk
klaim numerik `exp`, `iat`, `nbf` waktu ISO-8601 UTC beserta statusnya, mis.
`exp: 2026-01-01T03:00:00.000Z (kedaluwarsa 3 jam lalu)`, `(berlaku 2 hari
lagi)`, `(belum berlaku, 1 menit lagi)`. Klaim yang bukan angka tidak ditafsirkan;
`exp`/`nbf` yang ada tetapi bukan angka (termasuk `null`) memberi peringatan ke
stderr tanpa `--key`, atau membuat verifikasi gagal dengan `--key`.

## Opsi

- `-k, --key <secret>`: verifikasi HS256, HS384, atau HS512 (HMAC, dibandingkan
  dengan `timingSafeEqual`). Tidak boleh kosong.
- `--now <waktu>`: anggap sekarang = ISO-8601 (pemisah tanggal/waktu `T` atau
  spasi; tanpa zona selalu dianggap UTC, bukan zona lokal) atau epoch detik.
  Berguna agar hasil deterministik. Tanggal kalender harus valid:
  `2026-02-30` ditolak (exit 2), bukan digeser ke Maret.
- `-f, --file <path>`: baca token dari file (dengan `--sign`: payload).
- `--json`: keluaran satu dokumen JSON (lihat `## Keluaran JSON`).
- `-s, --sign`: mode tanda tangan (lihat di bawah).
- `--alg <alg>`, `--exp <durasi|epoch>`, `--iat`: hanya untuk `--sign`.

## Membuat token (`--sign`)

```bash
dt jwt --sign -k rahasia '{"sub":"42"}'
dt jwt -s -k rahasia --alg HS512 --exp 7d --iat -f payload.json
echo '{"role":"admin"}' | dt jwt -s -k rahasia --exp 15m
```

Payload (wajib objek JSON) dari argumen, `--file`, atau stdin. `--key` wajib.
Header selalu `{"alg":"<alg>","typ":"JWT"}`; `--alg` = `HS256` (default),
`HS384`, atau `HS512`. Encoding base64url tanpa padding. Keluaran: token kompak
saja.

- `--exp`: durasi ketat `<angka><s|m|h|d|w>` (`30s`, `15m`, `1h`, `7d`, `2w`)
  ditambahkan ke waktu sekarang (atau `--now`), atau epoch detik absolut
  (bilangan bulat tanpa satuan). Mengisi `exp` dan **menimpa** `exp` di payload.
- `--iat`: mengisi `iat` = sekarang (detik, dibulatkan ke bawah); menimpa `iat`
  di payload.
- `--now`: waktu acuan untuk `--exp` dan `--iat` (token menjadi deterministik).

Payload bukan objek JSON, `--key` hilang, `--alg`/`--exp` tidak valid: exit 2.
Token hasilnya bisa diperiksa dengan `dt jwt -k <secret> <token>`.

## Exit code

- `0`: sukses. Tanpa `--key`, token kedaluwarsa tetap exit 0.
- `1`: token rusak (bukan 3 segmen, base64url tidak valid, bukan JSON objek);
  dengan `--key`: `Signature tidak valid`, `alg` bukan HS* (termasuk `none`),
  token kedaluwarsa (`exp` <= sekarang), belum berlaku (`nbf` > sekarang),
  atau klaim `exp`/`nbf` yang ada tetapi bukan angka.
  Dengan `--key`, header, payload, dan waktu tetap dicetak dulu; error
  signature didahulukan atas kedaluwarsa.
  Angka tak hingga dari JSON (`1e999`, `-1e999`) tetap dibandingkan numerik:
  `exp: -Infinity` sudah kedaluwarsa, `exp: +Infinity` tidak pernah kedaluwarsa;
  `nbf: +Infinity` belum berlaku, `nbf: -Infinity` sudah berlaku.
- `2`: penggunaan salah (mis. `--now` tidak valid, `--sign` tanpa `--key`, payload bukan objek JSON, `--alg`/`--exp`/`--iat` tanpa `--sign`).

## Keluaran JSON

Dengan `--json`, stdout hanya satu dokumen JSON.

Decode/verifikasi:

- `header`, `payload`: objek hasil decode.
- `times`: array `{claim, iso, status}` untuk `exp`/`iat`/`nbf` numerik
  (`iso` = `null` bila di luar rentang waktu atau tak hingga; `status` = teks
  seperti mode biasa).
- `signature`: `{verified: boolean|null, algorithm?}`; `verified` `null` tanpa
  `--key`; `algorithm` (HS256/384/512) ada bila verifikasi dijalankan.
- `valid`: `null` tanpa `--key`; selain itu `true` hanya jika signature cocok,
  belum kedaluwarsa, `nbf` terlewati, dan `exp`/`nbf` bila ada berupa angka.

Kegagalan verifikasi tetap exit 1; JSON dicetak dulu, pesan error ke stderr.
Token rusak (tidak bisa di-decode): exit 1 tanpa JSON.

Dengan `--sign`: `{token, header, payload}` (payload setelah `exp`/`iat`
diterapkan).

## Catatan

- Tanpa `--key`, output menyatakan signature **tidak diverifikasi**: isi token
  bisa dipalsukan, jangan dipercaya untuk keputusan keamanan.
- `alg: none` tidak pernah dipercaya; algoritma asimetris (RS*, ES*, PS*) belum
  didukung dan ditolak bila memakai `--key`.
- Token wajib 3 segmen; segmen signature boleh kosong hanya untuk ditampilkan
  (tanpa `--key`).
- `--key` terlihat di daftar proses dan history shell.

[Kembali ke dokumentasi](README.md).
