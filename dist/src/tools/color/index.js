import { UsageError } from '../../errors.js';
import { choice, parseToolArgs, printJson, readInput } from '../../io.js';
import { contrast, nameOf, parseColor, ratioValue, toHex, toHsl, toHslString, toHwb, toHwbString, toRgbString, luminance, } from './color.js';
const FORMATS = ['hex', 'rgb', 'hsl', 'hwb', 'name'];
function help() {
    console.log(`dt color [options] <warna>

Konversi dan analisis warna: hex, rgb, hsl, hwb, nama CSS, luminans, dan kontras WCAG.
Warna dari argumen (digabung spasi) atau stdin. Format masukan:
  hex      #rgb  #rgba  #rrggbb  #rrggbbaa  (tanpa '#': hanya 6 atau 8 digit)
  rgb()    rgb(255, 127, 80)  rgba(255,127,80,.5)  rgb(255 127 80 / 50%)  rgb(100% 50% 0%)
  hsl()    hsl(16, 100%, 66%)  hsl(0.5turn 50% 50% / .3)  satuan hue: deg, grad, rad, turn
  hwb()    hwb(16 31% 0%)  (hanya sintaks spasi)
  nama     148 nama CSS (tanpa membedakan huruf besar/kecil) dan transparent
  -f, --format <jenis>    Cetak hanya satu nilai: ${FORMATS.join(', ')}
      --contrast <warna2> Rasio kontras antara <warna> dan <warna2>
      --json              Keluaran JSON
  -h, --help              Tampilkan bantuan

Pembulatan: kanal rgb dan hsl/hwb ke bilangan bulat; hsl/hwb dihitung dari rgb yang
sudah dibulatkan. Alpha dibulatkan sekali ke 3 desimal saat parsing; hex 8 digit bila
alpha hasil pembulatan < 1. Nama dengan '~' adalah nama terdekat (jarak RGB kuadrat);
alpha diabaikan untuk nama kecuali transparent (RGB hitam, alpha 0), luminans, dan kontras.
Rasio kontras dipotong (bukan dibulatkan) ke 2 desimal. Nilai AA/AAA memakai ambang
WCAG 2.x: teks normal 4.5 / 7, teks besar 3 / 4.5.
Contoh: dt color coral   |   dt color '#fff' --contrast '#767676'`);
}
const verdict = (ok) => (ok ? 'lulus' : 'gagal');
function contrastJson(c) {
    return {
        ratio: ratioValue(c.ratio),
        aa: c.aaNormal,
        aaa: c.aaaNormal,
        aaLarge: c.aaLarge,
        aaaLarge: c.aaaLarge,
    };
}
function normalVerdicts(c) {
    return `${ratioValue(c.ratio)}:1  AA ${verdict(c.aaNormal)}, AAA ${verdict(c.aaaNormal)}`;
}
function describe(input, c) {
    const hsl = toHsl(c);
    const hwb = toHwb(c);
    const name = nameOf(c);
    return {
        input,
        hex: toHex(c),
        rgb: { r: c.r, g: c.g, b: c.b },
        hsl,
        hwb: { h: hwb.h, w: hwb.w, b: hwb.b },
        css: { rgb: toRgbString(c), hsl: toHslString(c), hwb: toHwbString(c) },
        alpha: c.a,
        name: name.exact ? name.name : null,
        nearestName: name.name,
        luminance: Number(luminance(c).toFixed(6)),
        contrast: {
            white: contrastJson(contrast(c, { r: 255, g: 255, b: 255, a: 1 })),
            black: contrastJson(contrast(c, { r: 0, g: 0, b: 0, a: 1 })),
        },
    };
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, {
        format: { type: 'string', short: 'f' },
        contrast: { type: 'string' },
        json: { type: 'boolean' },
        help: { type: 'boolean', short: 'h' },
    });
    if (values.help) {
        help();
        return;
    }
    const format = values.format === undefined ? undefined : choice(values.format, FORMATS, 'hex', 'format');
    if (format && values.contrast !== undefined)
        throw new UsageError('--format tidak bisa dipakai bersama --contrast.');
    if (format && values.json)
        throw new UsageError('--json tidak bisa dipakai bersama --format.');
    const text = (await readInput(positionals, undefined, 'Warna:')).toString('utf8').trim();
    const color = parseColor(text);
    if (values.contrast !== undefined) {
        const other = parseColor(values.contrast);
        const c = contrast(color, other);
        if (values.json) {
            printJson({
                foreground: { input: text, hex: toHex(color) },
                background: { input: values.contrast.trim(), hex: toHex(other) },
                ratio: ratioValue(c.ratio),
                aa: c.aaNormal,
                aaa: c.aaaNormal,
                aaLarge: c.aaLarge,
                aaaLarge: c.aaaLarge,
            });
            return;
        }
        console.log(`Rasio kontras ${toHex(color)} vs ${toHex(other)}: ${ratioValue(c.ratio)}:1
  Teks normal  AA (4.5): ${verdict(c.aaNormal)}   AAA (7): ${verdict(c.aaaNormal)}
  Teks besar   AA (3):   ${verdict(c.aaLarge)}   AAA (4.5): ${verdict(c.aaaLarge)}`);
        return;
    }
    if (values.json) {
        printJson(describe(text, color));
        return;
    }
    const name = nameOf(color);
    const nameText = name.exact ? name.name : `~${name.name}`;
    switch (format) {
        case 'hex':
            console.log(toHex(color));
            return;
        case 'rgb':
            console.log(toRgbString(color));
            return;
        case 'hsl':
            console.log(toHslString(color));
            return;
        case 'hwb':
            console.log(toHwbString(color));
            return;
        case 'name':
            console.log(nameText);
            return;
    }
    console.log(`hex        ${toHex(color)}
rgb        ${toRgbString(color)}
hsl        ${toHslString(color)}
hwb        ${toHwbString(color)}
nama       ${nameText}
alpha      ${color.a}
luminans   ${luminance(color).toFixed(4)}
vs putih   ${normalVerdicts(contrast(color, { r: 255, g: 255, b: 255, a: 1 }))}
vs hitam   ${normalVerdicts(contrast(color, { r: 0, g: 0, b: 0, a: 1 }))}`);
}
export const colorTool = {
    name: 'color',
    description: 'Konversi warna (hex, rgb, hsl, hwb, nama CSS) dan kontras WCAG',
    category: 'Utilitas',
    run,
};
//# sourceMappingURL=index.js.map