# syncgit

Sinkronisasi satu arah commit dan tag antar remote Git, dengan pemetaan identitas
melalui mailmap. Memerlukan Git serta akses SSH/HTTPS ke remote yang digunakan.
Tidak bergantung pada API provider, Python, atau script shell.

## Interaktif

```bash
# Pilih command melalui menu
dt

# Buka wizard langsung
dt syncgit
```

Wizard meminta repo, remote sumber/tujuan, mailmap, dan branch lokal. Mode default
**preview**. Sinkron lokal/push membutuhkan konfirmasi akhir. Penghapusan tag
default **tidak**. Ctrl+C membatalkan prompt (exit 130). Wizard membutuhkan TTY.
Hanya branch lokal yang bisa dipilih; branch dari argumen yang tidak ada secara lokal
dilaporkan dan tidak ikut dipilih. Mailmap relatif dibaca dari repo yang dipilih di wizard.

Gunakan `--interactive` untuk wizard dengan nilai awal dari flag:

```bash
dt syncgit --interactive --repo /path/to/repo
```

## Command langsung / CI

Memberikan flag atau branch menjalankan command tanpa prompt.

```bash
# Preview; tidak mengubah commit/tag lokal atau remote tujuan
dt syncgit --repo /path/to/repo -n

# Sinkron lokal
dt syncgit --repo /path/to/repo

# Sinkron lokal lalu push
dt syncgit --repo /path/to/repo -p

# Repo/provider lain
dt syncgit --repo /path/to/work --source upstream --destination origin \
  --mailmap identities.txt -n main

# Bantuan lengkap
dt syncgit --help
```

## Opsi

| Opsi                     | Keterangan                                                                                   |
| ------------------------ | -------------------------------------------------------------------------------------------- |
| `--repo <path>`          | Repo kerja; default direktori saat ini.                                                      |
| `--source <remote>`      | Remote sumber; default `github`.                                                             |
| `--destination <remote>` | Remote push; default `$PUSH_REMOTE` (bila tidak kosong) atau `origin`.                       |
| `--mailmap <path>`       | Relatif ke repo atau absolut; default `mailmap.txt`.                                         |
| `-n`, `--dry-run`        | Preview tanpa commit/tag lokal atau push; fetch tetap dilakukan.                             |
| `-p`, `--push`           | Push branch dan tag setelah sinkronisasi lokal.                                              |
| `-d`, `--delete-tags`    | Hapus semua tag lokal yang tidak ada di sumber; dengan push, hapus nama yang sama di tujuan. |
| `-i`, `--interactive`    | Wizard dengan nilai awal dari opsi.                                                          |
| `-h`, `--help`           | Bantuan command.                                                                             |
| `branch ...`             | Branch yang disinkronkan; default branch HEAD remote sumber (mis. `main` atau `master`).     |

Tanpa argumen branch, branch default dibaca dari remote sumber (`git ls-remote --symref`),
sehingga membutuhkan akses ke remote. Wizard memilihnya otomatis bila branch lokal
dengan nama yang sama ada. Bila HEAD remote tidak terdeteksi, sebutkan branch secara eksplisit.

## Exit code

| Kode  | Arti                                                                                                     |
| ----- | -------------------------------------------------------------------------------------------------------- |
| `0`   | Sukses.                                                                                                  |
| `1`   | Gagal saat berjalan (Git, remote, working tree, konflik, push).                                          |
| `2`   | Penggunaan salah: command/opsi tidak dikenal, nilai bertentangan (mis. `--push` dengan sumber = tujuan). |
| `130` | Prompt dibatalkan dengan Ctrl+C.                                                                         |

## Pemetaan identitas

Contoh mailmap:

```text
Developer <developer@destination.example> <developer@source.example>
```

Remote sumber tetap memakai identitas asal. Commit/tag yang dibuat lokal dan
dikirim ke remote tujuan memakai identitas terpetakan; sumber tidak di-push.

## Perilaku dan batas keselamatan

- Working tree tracked harus bersih; operasi merge/rebase/cherry-pick belum selesai
  ditolak. Untracked file tidak ditolak, tetapi Git tetap bisa menolak checkout
  bila file itu akan tertimpa.
- Titik sinkron: commit sumber terbaru dengan tree, author date, dan subject yang
  sama seperti tip lokal, sehingga commit kosong sesudahnya tidak terlewat. Hash
  berubah tanpa perubahan isi dapat diterima. Bila tidak ada yang cocok, dipakai
  satu-satunya commit sumber dengan tree yang sama (dengan peringatan). Bila tree
  tidak ditemukan atau dimiliki lebih dari satu commit sumber, berhenti untuk
  pemeriksaan manual; tidak reset/force branch otomatis.
- Commit baru dibawa satu per satu. Pesan, tanggal author/committer dipertahankan;
  identitas lewat mailmap. Tree hasil harus identik sumber. Commit kosong didukung.
  Merge commit yang memerlukan mainline tidak otomatis diimpor.
- Konfigurasi mailmap repo tidak diubah. Signing otomatis dinonaktifkan karena
  objek hasil transformasi berbeda dari sumber.
- Tag sumber ditaruh di `refs/syncgit-tags/<source>/`, bukan `refs/tags/`.
  Tag dipetakan memakai tree + author date + subject. Kandidat ambigu/tidak ada
  padanan dilaporkan dan dilewati; exit sukses tidak berarti semua tag tersalin.
- Tag ringan tetap ringan; anotasi membawa pesan, tagger terpetakan dan tanggal.
  Signature asal dibuang karena tidak valid untuk objek baru. Target non-commit
  dilewati. Jika target dan jenis tag tidak berubah, perubahan pesan/tagger saja
  belum disalin.
- Dry-run masih fetch (mengubah tracking refs/namespace tag/cache objek), tetapi
  tidak checkout, membuat commit/tag lokal, menghapus tag lokal, atau push.
- Push branch fast-forward, lalu tag. Tag baru/tidak berubah tidak di-force;
  konflik tag tujuan ditolak. Tag yang dipindah menggunakan `--force-with-lease`
  yang dikunci ke nilai remote saat mulai. Tidak menimpa perubahan remote yang
  terjadi sesudah nilai itu dibaca. Push bukan transaksi atomik: bila push branch
  gagal, tag tidak di-push; tag yang ditolak tidak menghentikan push tag lainnya, dan
  semua kegagalan dilaporkan bersama di akhir (exit 1). Proteksi ref provider tetap berlaku.
- Sumber dan tujuan harus berbeda bila `--push` dipakai; tanpa push, tujuan tidak dipakai.
- Penghapusan tag mencakup **semua** tag lokal yang tidak ada di sumber, termasuk
  tag manual. Dengan push, nama tersebut juga dihapus di tujuan menggunakan lease.
  Tag yang hanya ada di tujuan (tidak ada lokal) tidak dihapus.
- HEAD awal dipulihkan setelah operasi normal maupun gagal, termasuk detached HEAD.
  Bila pemulihan sendiri gagal, error asli tetap dilaporkan. Saat konflik/perubahan
  belum di-commit, working tree dibiarkan; pesan error memberi perintah pemulihan
  lengkap beserta identitas/tanggal terpetakan. Tidak rollback commit/tag yang sudah
  berhasil dibuat.

## Verifikasi

Integration test memakai repo sementara tanpa jaringan. Mencakup pemetaan
identitas/tanggal/pesan/tree, hash rewrite, commit kosong, pemulihan branch/detached
HEAD, dry-run, tag anotasi/ringan/ambigu, retag dan push, opt-in deletion, penolakan
tag remote berbeda tanpa menghentikan push lain, dan dirty working tree.

```bash
npm test
```

[Kembali ke dokumentasi](README.md).
