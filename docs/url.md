# Utilitas url

```bash
dt url "https://user:pw@example.com:8443/a/b?x=1&x=2#frag"
dt url -q x "https://example.com/?x=1&x=2"   # 1 dan 2, satu per baris
dt url --json "https://example.com/?a=b"
echo "https://example.com" | dt url
```

Menguraikan URL absolut dengan parser WHATWG `URL` menjadi `protocol`,
`username`, `password`, `host`, `hostname`, `port`, `pathname`, `search`, `hash`,
dan `origin`; field kosong tidak ditampilkan. Port tanpa nilai eksplisit
ditampilkan `(default n)` untuk `http`, `https`, `ftp`, `ws`, `wss`. Di bawahnya
tabel `Query:` berisi pasangan key/value terdekode sesuai urutan; key berulang
menjadi baris terpisah.

Opsi:

- `-q, --query <key>`: hanya nilai terdekode key itu, satu per baris.
- `--json`: seluruh struktur sebagai satu dokumen JSON berindentasi 2 spasi,
  query berupa array `[key, value]`. Tidak bisa digabung dengan `--query`.
- `--show-password`: tampilkan password; default disamarkan `***`.
- `--file <path>`: baca URL dari berkas. Input juga bisa dari argumen atau stdin;
  spasi dan newline di tepi dibuang.

Exit: 0 sukses, 1 URL tidak valid atau key `--query` tidak ada, 2 penggunaan salah.

Catatan: skema wajib ada (`example.com/x` ditolak, tambahkan `https://`); URL
relatif tidak diselesaikan. Dekode query mengikuti form-urlencoded, jadi `+`
menjadi spasi. `pathname`, `search`, dan `hash` ditampilkan seperti hasil
normalisasi parser (belum terdekode). Pada URL tidak valid, userinfo (username
dan password) yang dipantulkan dalam pesan error disamarkan menjadi `***@`,
termasuk dengan `--show-password`; penyebab error tidak menyimpan input asli.
URL opaque seperti `mailto:user@example.com` tidak memiliki userinfo dan
alamatnya tidak disamarkan. Password pada URL tetap terlihat di daftar proses
dan history shell.

[Kembali ke dokumentasi](README.md).
