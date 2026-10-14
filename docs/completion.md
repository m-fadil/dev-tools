# completion

Mencetak skrip auto-complete untuk nama tool dan alias (argumen pertama). Opsi
tiap tool tidak ikut dilengkapi; gunakan `dt <tool> --help`.

```bash
dt completion bash
dt completion zsh
dt completion fish
```

## Pemasangan

```bash
# bash
echo 'source <(dt completion bash)' >> ~/.bashrc

# zsh: simpan sebagai _dt di direktori $fpath, lalu muat ulang
dt completion zsh > "${fpath[1]}/_dt"
autoload -U compinit && compinit

# fish
dt completion fish > ~/.config/fish/completions/dt.fish
```

Skrip dibuat dari registry saat dijalankan, jadi tool baru muncul setelah
skrip dibuat ulang. Shell selain `bash`, `zsh`, dan `fish` atau tanpa argumen
menghasilkan exit 2.

[Kembali ke dokumentasi](README.md).
