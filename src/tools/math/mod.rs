mod errors;
mod evaluate;
mod format;
mod functions;
mod lexer;
use crate::{
    errors::{CliError, Result, usage},
    io::{integer, parse, print_json, read_input},
    tool::Tool,
};
use clap::Parser;
use std::ffi::OsString;
pub static TOOL: Tool = Tool {
    name: "math",
    aliases: &[],
    description: "Hitung ekspresi matematika (operator, fungsi, konstanta)",
    category: "Matematika",
    run,
};
const HELP: &str = r#"dt math [options] <ekspresi...>

Hitung ekspresi matematika. Beberapa argumen digabung dengan spasi; tanpa argumen,
ekspresi dibaca dari stdin. Teks (argumen atau stdin) dipecah per baris menjadi
ekspresi terpisah (baris kosong dilewati), hasilnya satu baris per ekspresi. 'ans' = hasil baris sebelumnya.
  -p, --precision <n>  Jumlah desimal tetap, 0-15 (default: 12 digit signifikan)
      --scientific     Tampilkan dalam notasi ilmiah (dengan -p: desimal mantissa)
      --degrees        sin/cos/tan menerima derajat; asin/acos/atan/atan2 mengembalikan derajat
      --json           Keluaran JSON: {results: [{expression, result}]}
  -h, --help           Tampilkan bantuan

Angka: 12, 1.5, .5, 1e-3, 0xff, 1_000 (literal melampaui batas double, mis. 1e5000, = galat sintaks; 1e-5000 menjadi 0;
  heksadesimal di atas 2^53 kehilangan presisi). Operator: + - * / % ^ ** (pangkat, kanan-asosiatif,
-2^2 = -4), ! (faktorial 0-170 setelah operand), kurung. Perkalian implisit (2pi, 2(3))
tidak didukung; tulis 2*pi. % memakai tanda pembagi seperti JavaScript (-7 % 3 = -1).
Konstanta: pi e tau inf ans
Fungsi: sqrt cbrt abs sign floor ceil round trunc exp ln log(basis 10) log2 log10
  sin cos tan asin acos atan atan2 sinh cosh tanh min max pow hypot mod gcd lcm deg rad
  (round: setengah menjauhi nol; deg: radian->derajat; rad: derajat->radian)
Batas: ekspresi maks 10000 karakter, kedalaman maks 200.
Ekspresi diawali tanda minus perlu '--' agar tidak dibaca sebagai opsi.
Exit: 0 sukses, 1 galat hitung (bagi nol, sqrt negatif, hasil NaN/tak berhingga), 2 sintaks/penggunaan salah.
Contoh: dt math "2 * (3 + 4)^2"   |   dt math --degrees "sin(30)"   |   dt math -- -2^2"#;
#[derive(Parser)]
#[command(name="dt math",override_help=HELP,disable_version_flag=true,args_override_self=true)]
struct Args {
    #[arg(short = 'p', long)]
    precision: Option<String>,
    #[arg(long)]
    scientific: bool,
    #[arg(long)]
    degrees: bool,
    #[arg(long)]
    json: bool,
    positionals: Vec<String>,
}
fn run(args: Vec<OsString>) -> Result<()> {
    let a: Args = parse("math", args)?;
    if a.json && (a.precision.is_some() || a.scientific) {
        return Err(usage(
            "--json tidak dapat digabung dengan --precision atau --scientific.",
        ));
    }
    let p = integer(a.precision.as_deref(), -1, 0, 15, "precision")?;
    let p = if p < 0 { None } else { Some(p as usize) };
    let input = read_input(&a.positionals, None, "Ekspresi:")?;
    let text = String::from_utf8_lossy(&input);
    let lines: Vec<_> = text
        .split('\n')
        .enumerate()
        .map(|(i, s)| (i + 1, crate::tools::number::jstext::js_trim(s)))
        .filter(|(_, s)| !s.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(usage("Ekspresi wajib diisi, mis. dt math \"1 + 2\"."));
    }
    let mut ans = None;
    let mut results = Vec::new();
    for (number, expression) in &lines {
        let value = evaluate::evaluate(expression, a.degrees, ans).map_err(|e| {
            if lines.len() == 1 {
                e
            } else {
                match e {
                    CliError::Usage(s) => CliError::Usage(format!("baris {number}: {s}")),
                    CliError::Failed(s) => CliError::Failed(format!("baris {number}: {s}")),
                    e => e,
                }
            }
        })?;
        ans = Some(value);
        let formatted = format::format_number(value, p, a.scientific);
        if a.json {
            let result = if value.is_finite() {
                let n = formatted.parse::<f64>().unwrap_or(value);
                let s = crate::numfmt::js_number(n);
                serde_json::from_str::<serde_json::Value>(&s).map_err(|e| usage(e.to_string()))?
            } else {
                serde_json::json!(formatted)
            };
            results.push(serde_json::json!({"expression":expression,"result":result}));
        } else {
            println!("{formatted}");
        }
    }
    if a.json {
        print_json(&serde_json::json!({"results":results}));
    }
    Ok(())
}
