# semver

Operasi Semantic Versioning 2.0.0: parse, validasi, bandingkan, urutkan, naikkan versi, dan cocokkan dengan range. Alias: `sv`.

```bash
dt semver parse 1.2.3-beta.1+build.5
dt semver valid v1.2.3                  # valid (exit 0) / invalid (exit 1)
dt semver compare 1.0.0-rc.1 1.0.0      # -1
dt semver sort --reverse 1.10.0 1.2.0 1.9.0
dt semver bump preminor --preid beta 1.2.3   # 1.3.0-beta.0
dt semver satisfies 1.4.0 "^1.2 || >=3"      # true (exit 0) / false (exit 1)
```

## Perintah

- `parse <v>`: tampilkan `major`, `minor`, `patch`, `prerelease`, `build` (`-` jika kosong).
- `valid <v>`: cetak `valid` atau `invalid`. Exit 0 jika valid, 1 jika tidak. Tidak pernah exit 2 karena versi buruk.
- `compare <a> <b>`: cetak `-1`, `0`, atau `1`; exit 0.
- `sort <v...>`: urutkan naik, satu versi per baris. `--reverse` untuk turun. Pengurutan stabil: versi yang hanya beda
  metadata build tetap dalam urutan masukan.
- `bump <rilis> <v>`: naikkan versi. Rilis: `major`, `minor`, `patch`, `premajor`, `preminor`, `prepatch`, `prerelease`.
  `--preid <id>` memberi tag prerelease (hanya untuk rilis `pre*`; harus satu identifier alfanumerik yang memuat huruf
  atau `-`, bukan angka murni atau berisi titik).
- `satisfies <v> <range>`: cetak `true`/`false`; exit 0 jika cocok, 1 jika tidak.

## Aturan

- Tata bahasa SemVer 2.0.0 ketat: `MAJOR.MINOR.PATCH[-prerelease][+build]`, angka tanpa nol di depan (`01.2.3` dan
  `1.2.3-01` ditolak), ketiga bagian inti wajib, angka inti harus muat sebagai bilangan bulat aman.
- Satu awalan `v` diterima dan dibuang (`v1.2.3` = `1.2.3`); `vv1.2.3`, `=1.2.3`, dan spasi tidak diterima pada versi.
  Dalam range, `v` dan operator `=` boleh dipakai.
- Metadata build pada versi dan range harus berisi identifier tidak kosong dari huruf, angka, atau `-`,
  dipisahkan titik: `+..a` ditolak, sedangkan `+--` valid.
- `bump` exit 1 jika major/minor/patch hasil melebihi bilangan bulat aman (2^53-1).
- Versi tidak valid pada perintah selain `valid` menghasilkan error penggunaan (exit 2).
- Perbandingan mengikuti bagian 11 spesifikasi: inti dibandingkan numerik; versi tanpa prerelease lebih tinggi dari yang
  berprerelease; identifier numerik dibandingkan sebagai angka (berapa pun besarnya) dan lebih rendah dari alfanumerik;
  alfanumerik dibandingkan menurut urutan ASCII; himpunan prerelease yang lebih pendek lebih rendah bila awalannya sama.
  Metadata build diabaikan.

### Bump

- `major`/`minor`/`patch` pada versi prerelease hanya melepas tag bila memungkinkan: `1.2.3-beta.1` -> `patch` =
  `1.2.3`; `1.3.0-beta.1` -> `minor` = `1.3.0`; `2.0.0-rc.1` -> `major` = `2.0.0`.
- `premajor`/`preminor`/`prepatch` menaikkan bagian itu lalu menambah prerelease `0` (atau `<id>.0` dengan `--preid`).
- `prerelease` pada versi non-prerelease menaikkan patch dulu (`1.2.3` -> `1.2.4-0`). Pada versi prerelease, identifier
  numerik terakhir dinaikkan (`beta.1` -> `beta.2`), atau `.0` ditambahkan bila tak ada (`beta` -> `beta.0`).
  Identifier numerik prerelease dinaikkan tepat meskipun melampaui 2^53-1
  (`beta.9007199254740993` -> `beta.9007199254740994`).
- Dengan `--preid`: jika tag sama dan berhitung, hitungan dinaikkan; jika beda, mulai dari `<id>.0`.
- Metadata build dibuang pada hasil.

### Range

- Pembanding: `>=`, `<=`, `>`, `<`, `=` (tanpa operator = `=`), boleh dipisah spasi dari versi (`>= 1.2.3`).
- Caret `^1.2.3` (`>=1.2.3 <2.0.0`; `^0.2.3` = `>=0.2.3 <0.3.0`; `^0.0.3` = `>=0.0.3 <0.0.4`), tilde `~1.2.3` / `~>1.2.3`
  (`>=1.2.3 <1.3.0`), versi parsial dan x-range (`1.2.x`, `1.*`, `1`, `*`, `x`, juga `X`).
- Hyphen `1.2.3 - 2.3.4` (sisi parsial: `1.2 - 2.3` = `>=1.2.0 <2.4.0`).
- Spasi = AND, `||` = OR. Range kosong atau set kosong cocok dengan semua versi non-prerelease.
- Versi prerelease hanya cocok bila ada pembanding dalam himpunan yang sama (di antara `||`) dengan prerelease pada
  `[major, minor, patch]` yang sama: `1.2.3-beta.2` cocok dengan `^1.2.3-beta.1` tetapi tidak dengan `^1.2.0` atau `*`.
- Range tidak valid (`1.x.3`, `>>1`, `abc`) menghasilkan error penggunaan.

## Keluaran JSON

`--json` pada semua perintah: tepat satu dokumen JSON di stdout; exit code tidak berubah (`valid` invalid dan
`satisfies` salah tetap exit 1 dengan JSON tercetak).

- `parse`: `{ "input", "version", "major", "minor", "patch", "prerelease", "build" }`. `version` ternormalisasi (tanpa
  `v`); `prerelease` berupa array (identifier numerik berupa angka bila aman, selain itu string); `build` array string.
- `valid`: `{ "input", "valid", "version", "error" }`; `version` dan `error` bernilai `null` bila tidak relevan.
- `compare`: `{ "a", "b", "result" }` dengan `result` -1, 0, atau 1.
- `sort`: `{ "reverse", "versions" }`; `versions` array string ternormalisasi.
- `bump`: `{ "release", "preid", "from", "version" }`; `preid` `null` bila tidak diberikan.
- `satisfies`: `{ "version", "range", "satisfies" }`.

## Kode keluar

0 sukses (`valid` benar, `satisfies` cocok), 1 `valid`/`satisfies` bernilai salah atau hasil bump melebihi bilangan bulat
aman, 2 penggunaan salah (perintah/opsi tidak dikenal, versi atau range tidak valid).

[Kembali ke dokumentasi](README.md).
