# lorem

Membuat teks lorem ipsum untuk pengisi tampilan atau data uji.

```
dt lorem [-w <kata> | -s <kalimat> | -p <paragraf>] [--seed n] [--classic] [--json]
```

| Opsi                   | Keterangan                                                             |
| ---------------------- | ---------------------------------------------------------------------- |
| `-w, --words <n>`      | Jumlah kata, 1-5000                                                    |
| `-s, --sentences <n>`  | Jumlah kalimat, 1-500                                                  |
| `-p, --paragraphs <n>` | Jumlah paragraf, 1-100 (default: 3)                                    |
| `--seed <n>`           | Seed bilangan bulat 0-4294967295; seed sama menghasilkan teks sama     |
| `--classic`            | Mulai dengan "Lorem ipsum dolor sit amet, consectetur adipiscing elit" |
| `--json`               | Keluaran JSON                                                          |

Pilih tepat satu satuan. Kalimat terdiri atas 5-15 kata, diawali huruf kapital, diakhiri titik, dan kadang berkoma.
Paragraf terdiri atas 3-7 kalimat; antarparagraf dipisah satu baris kosong. Tanpa `--seed`, seed diambil dari
CSPRNG sistem operasi. Nilai di luar batas menghasilkan kode keluar 2.

```
dt lorem -w 20 --seed 1
dt lorem -p 2 --classic
```

## Keluaran JSON

```json
{ "unit": "paragraphs", "count": 3, "text": "..." }
```

- `unit`: `words`, `sentences`, atau `paragraphs`
- `count`: jumlah yang diminta
- `text`: teks hasil

[Kembali ke dokumentasi](README.md).
