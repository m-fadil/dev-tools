# escape

Escape dan unescape teks untuk JSON, HTML, URL, regex, dan shell.

```
dt escape <mode> [-d] [--quotes] [--ascii] [--plus] [-f <path>] [--json] [teks]
```

Teks dibaca dari argumen, `--file`, atau stdin (pada terminal tanpa input: prompt). Input harus UTF-8 valid.

| Mode    | Escape                                                  | `--decode`                                          |
| ------- | ------------------------------------------------------- | --------------------------------------------------- |
| `json`  | Isi string JSON (RFC 8259)                              | Ya, dengan atau tanpa tanda kutip                   |
| `html`  | `& < > " '` menjadi `&amp; &lt; &gt; &quot; &#39;`      | Ya, entitas bernama umum dan numerik               |
| `url`   | Percent-encoding komponen URL (RFC 3986), huruf besar   | Ya                                                  |
| `regex` | Escape untuk crate `regex` Rust (`regex::escape`)       | Tidak (kode keluar 2)                               |
| `shell` | Kutip tunggal POSIX, `'` menjadi `'\''`; kosong menjadi `''` | Tidak (kode keluar 2)                          |

| Opsi              | Keterangan                                                          |
| ----------------- | ------------------------------------------------------------------- |
| `-d, --decode`    | Balikkan proses (unescape)                                          |
| `-f, --file`      | Baca isi file                                                       |
| `--quotes`        | `json`: bungkus hasil dengan tanda kutip                            |
| `--ascii`         | `json`: escape karakter non-ASCII sebagai `\uXXXX` (pasangan surrogate untuk karakter di luar BMP) |
| `--plus`          | `url --decode`: ubah `+` menjadi spasi                              |
| `--json`          | Keluaran JSON                                                       |

`--quotes`/`--ascii` hanya untuk `json` tanpa `--decode`, dan `--plus` hanya untuk `url --decode`; selain itu kode keluar 2.

## Perilaku

- **json**: `" \ \n \r \t \b \f` ditulis singkat, kontrol lain `\u00XX`; `/` dan non-ASCII dibiarkan. Decode memvalidasi escape (`\/` diterima), menolak tanda kutip tanpa escape, karakter kontrol mentah, `\u` rusak, dan surrogate tanpa pasangan.
- **html**: decode menangani `amp lt gt quot apos nbsp copy reg hellip mdash ndash`, `&#NN;`, dan `&#xHH;` (hanya titik kode valid, bukan 0). Entitas lain dibiarkan apa adanya.
- **url**: karakter tak-cadangan (`A-Z a-z 0-9 - . _ ~`) dipertahankan; sisanya `%XX` per byte UTF-8. Decode gagal pada `%` tidak valid atau hasil yang bukan UTF-8.
- Saat `--decode`, satu akhir baris (`\n`/`\r\n`) pada input dibuang. Saat escape, input apa adanya.
- Hasil dicetak dengan satu newline di akhir.
- Kode keluar: 0 berhasil, 1 gagal (input tidak valid), 2 salah pakai.

## Contoh

```
$ dt escape json 'kata "ini" \ tab	x'
kata \"ini\" \\ tab\tx

$ dt escape json --ascii --quotes 'é😀'
"\u00e9\ud83d\ude00"

$ dt escape json -d '"a\u00e9\ud83d\ude00\n"'
aé😀

$ dt escape html "<a href=\"x\">Tom & 'Jerry'</a>"
&lt;a href=&quot;x&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/a&gt;

$ dt escape html -d '&lt;p&gt;&copy; &#x1F600; &bogus;'
<p>© 😀 &bogus;

$ dt escape url 'a b&c=é'
a%20b%26c%3D%C3%A9

$ dt escape url -d --plus 'a+b%20c'
a b c

$ dt escape regex 'a.b*(c)'
a\.b\*\(c\)

$ dt escape shell "it's"
'it'\''s'
```

Kesalahan:

```
$ dt escape json -d '\ud83d'
Surrogate tinggi tanpa pasangan pada posisi 1.        (kode 1)

$ dt escape url -d '%FF'
Hasil decode bukan UTF-8 yang valid.                   (kode 1)

$ dt escape shell -d x
Mode shell tidak mendukung --decode.                   (kode 2)
```

## Keluaran JSON

```
$ dt escape url --json 'a b'
{
  "mode": "url",
  "direction": "encode",
  "output": "a%20b"
}
```

- `mode`: `json`, `html`, `url`, `regex`, atau `shell`
- `direction`: `encode` atau `decode`
- `output`: hasil (input tidak diulang, seperti `dt encode`)

[Kembali ke dokumentasi](README.md).
