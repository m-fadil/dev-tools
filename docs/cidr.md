# cidr

Hitung detail subnet IPv4 dan IPv6 dengan aritmetika BigInt dan parsing ketat.

```bash
dt cidr 192.168.1.10/24
dt cidr 10.0.0.0 255.255.255.0           # bentuk "ip netmask" (IPv4)
dt cidr 2001:db8::1/32
dt cidr --contains 10.0.0.77 10.0.0.0/24   # yes / no
dt cidr --split 26 10.0.0.0/24             # daftar subnet /26
dt cidr --json 10.0.0.1/31
```

IPv4 menampilkan alamat, jaringan, prefix, netmask, wildcard, broadcast, host
pertama/terakhir, total alamat, host usable, dan penanda `private` (RFC 1918),
`loopback`, `link-local`. Penanda berlaku untuk subnet, bukan alamat host input:
hanya `true` bila seluruh rentang subnet (network sampai broadcast) berada di
dalam blok terkait (10/8, 172.16/12, 192.168/16; 127/8; 169.254/16). Jadi
`10.1.2.3/8` private, tetapi `10.1.2.3/0`, `10.0.0.0/7`, dan
`192.168.1.1/8` bukan. Di keluaran teks penanda muncul di baris `Seluruh subnet`.
Host usable: `/31` = 2 (RFC 3021,
tanpa network/broadcast), `/32` = 1, lainnya total - 2. IPv6 menampilkan jaringan,
prefix, alamat pertama/terakhir, dan total alamat (string desimal).

## Opsi

- `--contains <ip>`: cetak `yes`/`no`. Versi IP berbeda dengan subnet dianggap `no`
  (termasuk `::ffff:a.b.c.d` terhadap subnet IPv4).
- `--split <prefix>`: daftar subnet dengan panjang prefix itu, satu per baris.
  Galat bila lebih pendek dari prefix saat ini, melebihi panjang alamat, atau
  menghasilkan lebih dari 4096 subnet. Tidak bisa digabung dengan `--contains`.
- `--json`: keluaran JSON.

## Parsing

- Prefix wajib (`ip/prefix` atau `ip netmask`); bentuk netmask hanya IPv4 dan
  netmask harus kontigu.
- IPv4 ketat: empat oktet 0-255; oktet berawalan nol (`010`) ditolak karena
  ambigu (oktal/desimal). Prefix tanpa nol di depan.
- IPv6: kompresi `::` maksimal satu kali dan mewakili minimal satu grup, 1-4 digit
  hex per grup, ekor IPv4 (`::ffff:1.2.3.4`) diterima, zone id (`%eth0`) ditolak.
  Keluaran memakai bentuk ringkas RFC 5952 (huruf kecil): rentang nol terpanjang
  dikompresi, rentang pertama dipilih jika sama panjang, dan satu grup nol tidak
  dikompresi. IPv4-mapped ditulis `::ffff:a.b.c.d`; ekor IPv4 lainnya ditulis
  sebagai grup hex (`::1.2.3.4` menjadi `::102:304`).

## Kode exit

- `0` sukses; dengan `--contains`: ip ada di subnet.
- `1` hanya dengan `--contains`: ip tidak ada di subnet.
- `2` argumen, alamat, atau prefix tidak valid.

## Keluaran JSON

Default (IPv4): `version`, `address`, `network`, `prefix`, `cidr`, `netmask`, `wildcard`,
`broadcast`, `firstHost`, `lastHost`, `totalAddresses` (string), `usableHosts` (string),
`private`, `loopback`, `linkLocal` (boolean).
IPv6: `version`, `address`, `network`, `prefix`, `cidr`, `first`, `last`, `totalAddresses` (string).

`--contains`: `{ network, address, contains }` (`contains` boolean).
`--split`: `{ network, prefix, count, subnets }` (`prefix` = panjang baru, `subnets` = array string CIDR).

[Kembali ke dokumentasi](README.md).
