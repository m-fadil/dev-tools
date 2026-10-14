# math

Kalkulator ekspresi matematika. Penguraian dilakukan dengan tokenizer dan parser rekursif buatan sendiri; tidak ada `eval`, `Function`, atau `vm`, sehingga input tidak dapat menjalankan kode.

```
dt math [opsi] <ekspresi...>
```

Beberapa argumen digabung dengan spasi. Teks hasil (argumen atau stdin) dipecah per baris: setiap baris tak kosong menjadi satu ekspresi (argumen tunggal `"1\n2"` menjadi dua ekspresi, bukan satu), hasil dicetak satu baris per ekspresi.

## Opsi

| Opsi                  | Keterangan                                                                             |
| --------------------- | -------------------------------------------------------------------------------------- |
| `-p`, `--precision n` | Jumlah desimal tetap, 0-15. Default: pembulatan 12 digit signifikan                    |
| `--scientific`        | Notasi ilmiah (`1.5e+3`); bersama `-p`, `n` = jumlah desimal mantissa                  |
| `--degrees`           | `sin`/`cos`/`tan` menerima derajat; `asin`/`acos`/`atan`/`atan2` mengembalikan derajat |
| `--json`              | Keluaran JSON (lihat bawah); tidak dapat digabung dengan `-p`/`--scientific`           |
| `-h`, `--help`        | Bantuan                                                                                |

Ekspresi yang diawali tanda minus harus didahului `--` agar tidak dibaca sebagai opsi: `dt math -- -2^2`.

## Sintaks

- **Angka**: `12`, `1.5`, `.5`, `1e-3`, heksadesimal `0xff`, pemisah `_` di antara digit (`1_000_000`). Literal yang melampaui batas double (`1e5000`) adalah galat sintaks (kode 2); `1e-5000` menjadi `0`; heksadesimal di atas 2^53 kehilangan presisi karena dibaca sebagai double.
- **Operator** (dari yang terkuat): `!` (faktorial postfix, bilangan bulat 0-170), `^` / `**` (pangkat, asosiatif kanan, mengikat lebih kuat dari minus unary di kirinya: `-2^2 = -4`, `2^3^2 = 512`, eksponen boleh negatif: `2^-1`), minus/plus unary, `* / %`, `+ -`. Kurung `( )` mengubah urutan.
- `%` adalah sisa bagi gaya JavaScript: tanda mengikuti yang dibagi (`-7 % 3 = -1`). `mod(a, b)` sama.
- **Konstanta**: `pi`, `e`, `tau`, `inf`, dan `ans` (hasil baris sebelumnya pada input banyak baris). Nama tidak membedakan huruf besar/kecil.
- **Perkalian implisit tidak didukung**: `2pi` atau `2(3)` ditolak; tulis `2*pi`, `2*(3)`.
- **Fungsi**: `sqrt cbrt abs sign floor ceil round trunc exp ln log log2 log10 sin cos tan asin acos atan atan2 sinh cosh tanh min max pow hypot mod gcd lcm deg rad`. `log` berbasis 10; `round` membulatkan setengah menjauhi nol; `min`/`max`/`hypot` menerima satu argumen atau lebih, `atan2`, `pow`, `mod`, `gcd`, `lcm` tepat dua, sisanya satu. `gcd`/`lcm` hanya untuk bilangan bulat aman. `deg` mengubah radian ke derajat, `rad` sebaliknya (tidak dipengaruhi `--degrees`). Fungsi trigonometri memakai radian kecuali `--degrees`; pada derajat hasilnya tepat di kelipatan 90 (`sin(180) = 0`), `sin`/`cos` di kelipatan 30 yang hasilnya 0, ±0.5, ±1 (`sin(30) = 0.5`, `cos(60) = 0.5`), dan `tan` di 45° + k·180° (`1`) serta 135° + k·180° (`-1`). `tan(90° + k·180°)` adalah galat hitung.

Batas: panjang ekspresi maksimum 10.000 karakter, kedalaman tingkat (kurung, argumen fungsi, rantai minus/pangkat) maksimum 200.

## Format hasil

Hasil non-bulat dibulatkan ke 12 digit signifikan agar derau floating point tersembunyi (`0.1+0.2` tercetak `0.3`). Hasil bulat tidak dibulatkan: bilangan bulat tercetak utuh hingga di bawah 1e21 (`2^64 = 18446744073709551616`), sehingga pembagian yang tidak tepat pada angka besar menampakkan derau double (`1e20/3` = `33333333333333331968`); selebihnya dalam bentuk eksponen. Dengan `-p n`, hasil ≥ 1e21 tetap dicetak desimal tetap (`-p 2 1e21` = `1000000000000000000000.00`); pada besaran itu semua double adalah bilangan bulat. Hasil tak berhingga hanya boleh berasal dari `inf` dan dicetak `inf` / `-inf`.

## Contoh

```
$ dt math "2 * (3 + 4)^2"
98
$ dt math --degrees "sin(30)"
0.5
$ printf '10 / 4\nans * 2\n' | dt math
2.5
5
$ dt math -p 4 "1/3"
0.3333
$ dt math "2pi"
Perkalian implisit tidak didukung; tulis '*' secara eksplisit (ditemukan 'pi') (kolom 2)
```

## Kode keluar

- `0` sukses.
- `1` galat hitung: pembagian/modulo dengan nol, `sqrt` bilangan negatif, `ln`/`log` untuk x ≤ 0, argumen `asin`/`acos` di luar [-1, 1], faktorial di luar 0-170, hasil NaN atau tak berhingga (kecuali dari `inf`), `ans` tanpa hasil sebelumnya.
- `2` sintaks atau penggunaan salah: token/karakter tak dikenal, literal angka terlalu besar (`1e5000`), kurung tak seimbang, identifier tidak dikenal, jumlah argumen fungsi salah, perkalian implisit, melewati batas panjang/kedalaman, opsi tidak valid. Ekspresi diurai sepenuhnya sebelum galat hitung dilaporkan, sehingga galat sintaks selalu menang (`1/0 + (` berkode 2).

Pesan galat menyebut kolom (1-based) masalahnya dan, pada input banyak baris, nomor baris (`baris 2: ...`). Ekspresi di-echo dengan karakter kontrol di-escape (tab menjadi `\t`) agar tanda `^` sejajar. Hasil baris-baris sebelum galat sudah tercetak (kecuali dengan `--json`).

## Keluaran JSON

Dengan `--json`, stdout berisi tepat satu dokumen JSON:

```json
{
  "results": [
    { "expression": "0.1+0.2", "result": 0.3 },
    { "expression": "inf", "result": "inf" }
  ]
}
```

- `results`: array, satu elemen per ekspresi (urutan input).
- `results[].expression`: teks ekspresi (sudah di-trim).
- `results[].result`: angka (dibulatkan 12 digit signifikan untuk hasil non-bulat); untuk hasil tak berhingga berupa string `"inf"` atau `"-inf"`. Angka JSON adalah double JavaScript, jadi bilangan bulat besar tidak sama dengan keluaran teks: `2^64` bernilai `18446744073709552000` di JSON, sedangkan teks mencetak `18446744073709551616`.

Jika ada ekspresi yang galat, tidak ada yang dicetak ke stdout; galat ke stderr dan kode keluar tetap 1/2.

[Kembali ke dokumentasi](README.md).
