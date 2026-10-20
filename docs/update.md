# update

Memeriksa dan memasang rilis `dt` dari GitHub Releases. Alurnya sama dengan `install.sh`: unduh arsip dan `.sha256`,
verifikasi SHA-256, lalu ganti binary `dt` yang sedang berjalan.

```
dt update [--check] [--version <tag>] [--force] [--json]
```

| Opsi              | Keterangan                                                                                       |
| ----------------- | ------------------------------------------------------------------------------------------------ |
| `--check`         | Hanya laporkan versi saat ini, versi terbaru, dan ketersediaan pembaruan; tidak memasang apa pun |
| `--version <tag>` | Pasang versi tertentu (`v1.2.3` atau `1.2.3`), termasuk prerelease dan versi yang lebih lama     |
| `--force`         | Pasang ulang meski versinya sama atau sudah lebih baru                                           |
| `--json`          | Keluaran JSON                                                                                    |

## Cara kerja

1. **Versi tujuan.** Tanpa `--version`, rilis stabil terbaru dicari dari tujuan redirect `<rilis>/latest`
   (`curl -fsSLI -o /dev/null -w '%{url_effective}'`, persis seperti `install.sh`). Prerelease tidak pernah dipilih
   otomatis; pasang lewat `--version vX.Y.Z-rc.1`.
2. **Perbandingan.** Versi dibandingkan dengan aturan SemVer (`dt semver`). Tanpa `--version`, pembaruan hanya dipasang
   jika versi terbaru lebih tinggi dari versi saat ini. Dengan `--version`, versi yang berbeda selalu dipasang (juga
   versi yang lebih lama); versi yang sama dilewati kecuali ada `--force`.
3. **Unduh.** `dt-<tag>-<target>.tar.gz` dan `.sha256`-nya diunduh ke folder sementara.
4. **Verifikasi.** SHA-256 arsip dihitung di dalam proses dan dibandingkan dengan isi `.sha256`. Tidak cocok: proses
   dibatalkan dan tidak ada yang diganti.
5. **Ekstrak.** Berkas `dt-<tag>-<target>/dt` diekstrak langsung ke berkas sementara di folder yang
   sama dengan binary tujuan, lalu diberi mode `0755`.
6. **Ganti.** Binary yang sedang berjalan (symlink diikuti sampai berkas aslinya) diganti dengan `rename` atomik.

Folder sementara dan berkas perantara selalu dibersihkan, juga saat gagal; binary lama tidak pernah dibiarkan setengah
tertulis.

## Target

Dipilih saat kompilasi, sama dengan `install.sh`:

| Platform | Aset                                                                  |
| -------- | --------------------------------------------------------------------- |
| Linux    | `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl` (`.tar.gz`) |

Arsitektur atau OS lain menghasilkan error (exit 1). Binary hasil `cargo install` ikut diganti dengan build rilis
(di Linux: build musl), bukan dibangun ulang dari sumber.

## Kebutuhan

`dt` tidak membawa klien HTTP. Unduhan dilakukan lewat `curl` (diutamakan) atau
`wget`. Jika keduanya tidak ada, `dt update` gagal dengan pesan yang jelas (exit 1). `--check --version <tag>` tidak
memakai jaringan sama sekali sehingga tidak butuh keduanya.

`DT_RELEASES_URL` mengganti dasar URL rilis (default `https://github.com/m-fadil/dev-tools/releases`), sama dengan
`install.sh`; berguna untuk cermin internal dan pengujian. Garis miring di ujung diabaikan.

## Contoh

Contoh berikut memakai cermin lokal (`DT_RELEASES_URL=http://127.0.0.1:8765`) dengan rilis palsu `v9.9.9`.

```
$ dt update --check
Versi saat ini : X.Y.Z
Versi terbaru  : 9.9.9
Pembaruan tersedia. Jalankan: dt update

$ dt update --check --version v1.0.0
Versi saat ini : X.Y.Z
Versi terbaru  : 1.0.0
dt sudah versi terbaru.

$ dt update
Memperbarui dt X.Y.Z -> 9.9.9 (x86_64-unknown-linux-musl)
Berhasil diperbarui ke v9.9.9
Terpasang: /tmp/smoke/bin/dt

$ dt update --version X.Y.Z
dt sudah versi terbaru (X.Y.Z).
```

Checksum tidak cocok (exit 1, binary tidak berubah):

```
$ dt update --version v9.9.9
Memperbarui dt X.Y.Z -> 9.9.9 (x86_64-unknown-linux-musl)
Checksum tidak cocok untuk dt-v9.9.9-x86_64-unknown-linux-musl.tar.gz.
```

Folder tujuan tidak bisa ditulis (exit 1; gagal sebelum mengunduh):

```
$ dt update --version v9.9.9
Memperbarui dt X.Y.Z -> 9.9.9 (x86_64-unknown-linux-musl)
Tidak bisa menulis ke /tmp/smoke/bin: Permission denied (os error 13). Jalankan ulang dengan izin yang sesuai atau pasang ulang dengan install.sh.
```

## Keluaran JSON

```json
{
  "current": "X.Y.Z",
  "latest": "9.9.9",
  "available": true,
  "updated": true,
  "target": "x86_64-unknown-linux-musl"
}
```

- `current`: versi `dt` yang berjalan (tanpa `v`)
- `latest`: versi tujuan tanpa `v` (rilis stabil terbaru, atau nilai `--version`)
- `available`: `true` bila versi tujuan lebih tinggi dari versi saat ini
- `updated`: `true` hanya bila binary benar-benar diganti (selalu `false` dengan `--check`)
- `target`: triple platform rilis; ada pada pembaruan yang dipasang dan pada `--check` di platform yang didukung,
  dihilangkan bila tidak ada pemasangan dan tidak ada yang dilaporkan

Saat gagal, tidak ada JSON yang dicetak; pesan error ada di stderr.

## Kode keluar

0 sukses (termasuk `--check` apa pun hasilnya, dan sudah versi terbaru), 1 gagal (jaringan tidak terjangkau dengan penyebabnya, tidak ada rilis stabil, unduhan atau
checksum gagal, arsip tanpa `dt`, folder tidak bisa ditulis, curl/wget tidak ada, platform tidak didukung), 2 penggunaan
salah (opsi tidak dikenal, versi bukan SemVer).

[Kembali ke dokumentasi](README.md).
