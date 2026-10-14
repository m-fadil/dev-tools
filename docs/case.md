# case

Ubah gaya penulisan teks atau identifier.

```bash
dt case -t snake parseURL2Json      # parse_url_2_json
dt case -t camel "hello world"      # helloWorld
echo "HTTPServerError" | dt case    # semua gaya, berlabel
dt case -t kebab --file nama.txt
```

Teks multibaris: tiap baris dikonversi sendiri (baris kosong dipertahankan;
satu newline di akhir input tidak dihitung sebagai baris).

## Opsi

- `-t, --to <gaya>`: `camel`, `pascal`, `snake`, `screaming` (SCREAMING_SNAKE),
  `kebab`, `train` (Train-Case), `dot`, `path` (a/b/c), `title`, `sentence`,
  `lower`, `upper`, atau `all` (default: semua gaya dengan label).
- `-f, --file <path>`: baca teks dari file (tidak bersama teks posisional).
- `--json`: keluaran JSON.

## Pemisahan kata

Pemisah: semua karakter selain huruf/angka Unicode (spasi, `_`, `-`, `.`, `/`,
tanda baca). Apostrof dibuang (`don't` menjadi `dont`). Batas kata tambahan:
`aB` (camelCase), akhir akronim (`HTTPServer` menjadi `http server`), dan
pergantian huruf/angka (`parseURL2Json` menjadi `parse url 2 json`; `utf8`
menjadi `utf 8`). Huruf Unicode (`ÉcoleNormale`, `привет`) ditangani.

## Keluaran JSON

```json
{ "style": "snake", "lines": [{ "input": "fooBar", "output": "foo_bar" }] }
```

- `style`: gaya yang dipilih, atau `"all"`.
- `lines[].input`: baris asli.
- `lines[].output`: string hasil; dengan `all` berupa objek `{ <gaya>: string }` untuk 12 gaya.

[Kembali ke dokumentasi](README.md).
