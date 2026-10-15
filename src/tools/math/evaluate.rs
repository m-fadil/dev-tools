use super::{
    errors::error,
    functions,
    lexer::{Token, lex},
};
use crate::errors::{CliError, Result};
struct Evaluator<'a> {
    src: &'a str,
    tok: Token,
    next: usize,
    depth: usize,
    pending: Option<CliError>,
    degrees: bool,
    ans: Option<f64>,
}
impl Evaluator<'_> {
    fn advance(&mut self) -> Result<()> {
        (self.tok, self.next) = lex(self.src, self.next)?;
        Ok(())
    }
    fn op(&self, ops: &[&str]) -> bool {
        self.tok.kind == 2 && ops.contains(&self.tok.text.as_str())
    }
    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > 200 {
            Err(error(
                self.src,
                self.tok.col,
                "Ekspresi terlalu dalam (maksimum 200 tingkat)",
                false,
            ))
        } else {
            Ok(())
        }
    }
    fn unexpected(&self, expected: Option<&str>) -> CliError {
        let suffix = expected.map(|e| format!(", diharapkan {e}")).unwrap_or_default();
        let t = &self.tok;
        let msg = if t.kind == 3 {
            format!("Ekspresi berakhir tiba-tiba{suffix}")
        } else if t.kind == 0 || t.kind == 1 || t.text == "(" {
            format!(
                "Perkalian implisit tidak didukung; tulis '*' secara eksplisit (ditemukan '{}')",
                t.text
            )
        } else {
            format!("Token tidak terduga '{}'{suffix}", t.text)
        };
        error(self.src, t.col, msg, false)
    }
    fn compute(&mut self, col: usize, inputs: &[f64], result: std::result::Result<f64, String>) -> f64 {
        if self.pending.is_some() {
            return 0.;
        }
        let result = result.and_then(|v| {
            if v.is_nan() {
                Err("Hasil tidak terdefinisi (NaN)".into())
            } else if !v.is_finite() && inputs.iter().all(|x| x.is_finite()) {
                Err("Hasil melampaui batas bilangan (tak berhingga)".into())
            } else {
                Ok(v)
            }
        });
        match result {
            Ok(v) => v,
            Err(e) => {
                self.pending = Some(error(self.src, col, e, true));
                0.
            }
        }
    }
    fn expr(&mut self) -> Result<f64> {
        let mut l = self.term()?;
        while self.op(&["+", "-"]) {
            let t = self.tok.clone();
            self.advance()?;
            let r = self.term()?;
            l = self.compute(t.col, &[l, r], Ok(if t.text == "+" { l + r } else { l - r }));
        }
        Ok(l)
    }
    fn term(&mut self) -> Result<f64> {
        let mut l = self.unary(false)?;
        while self.op(&["*", "/", "%"]) {
            let t = self.tok.clone();
            self.advance()?;
            let r = self.unary(false)?;
            let v = match t.text.as_str() {
                "*" => Ok(l * r),
                "%" => functions::remainder(l, r),
                _ => {
                    if r == 0. {
                        Err("Pembagian dengan nol".into())
                    } else {
                        Ok(l / r)
                    }
                }
            };
            l = self.compute(t.col, &[l, r], v);
        }
        Ok(l)
    }
    fn unary(&mut self, _exponent: bool) -> Result<f64> {
        if !self.op(&["-", "+"]) {
            return self.power();
        }
        self.enter()?;
        let negative = self.tok.text == "-";
        self.advance()?;
        let v = self.unary(false)?;
        self.depth -= 1;
        Ok(if negative { -v } else { v })
    }
    fn power(&mut self) -> Result<f64> {
        let mut v = self.primary()?;
        while self.op(&["!"]) {
            let col = self.tok.col;
            v = self.compute(col, &[v], functions::factorial(v));
            self.advance()?;
        }
        if self.op(&["^", "**"]) {
            let col = self.tok.col;
            self.advance()?;
            self.enter()?;
            let r = self.unary(true)?;
            self.depth -= 1;
            v = self.compute(col, &[v, r], functions::power(v, r));
        }
        Ok(v)
    }
    fn primary(&mut self) -> Result<f64> {
        let t = self.tok.clone();
        if t.kind == 0 {
            self.advance()?;
            return Ok(t.value);
        }
        if self.op(&["("]) {
            self.enter()?;
            self.advance()?;
            let v = self.expr()?;
            self.depth -= 1;
            if !self.op(&[")"]) {
                return Err(self.unexpected(Some("')'")));
            }
            self.advance()?;
            return Ok(v);
        }
        if t.kind != 1 {
            return Err(self.unexpected(Some("angka, konstanta, fungsi, atau '('")));
        }
        let name = t.text.to_ascii_lowercase();
        let constant = match name.as_str() {
            "pi" => Some(std::f64::consts::PI),
            "e" => Some(std::f64::consts::E),
            "tau" => Some(std::f64::consts::TAU),
            "inf" => Some(f64::INFINITY),
            "ans" => {
                if self.ans.is_none() && self.pending.is_none() {
                    self.pending = Some(error(
                        self.src,
                        t.col,
                        "'ans' belum ada: belum ada hasil sebelumnya",
                        true,
                    ));
                }
                Some(self.ans.unwrap_or(0.))
            }
            _ => None,
        };
        if let Some(v) = constant {
            self.advance()?;
            return Ok(v);
        }
        let (min, max) = functions::arity(&name)
            .ok_or_else(|| error(self.src, t.col, format!("Identifier tidak dikenal: {}", t.text), false))?;
        self.advance()?;
        if !self.op(&["("]) {
            return Err(error(
                self.src,
                self.tok.col,
                format!("'(' diharapkan setelah fungsi {name}"),
                false,
            ));
        }
        self.enter()?;
        self.advance()?;
        let mut args = Vec::new();
        if !self.op(&[")"]) {
            loop {
                args.push(self.expr()?);
                if !self.op(&[","]) {
                    break;
                }
                self.advance()?;
            }
        }
        if !self.op(&[")"]) {
            return Err(self.unexpected(Some("',' atau ')'")));
        }
        self.depth -= 1;
        self.advance()?;
        if args.len() < min || args.len() > max {
            let range = if min == max {
                min.to_string()
            } else if max == usize::MAX {
                format!("minimal {min}")
            } else {
                format!("{min}-{max}")
            };
            return Err(error(
                self.src,
                t.col,
                format!("Fungsi {name} membutuhkan {range} argumen, diberikan {}", args.len()),
                false,
            ));
        }
        let result = functions::call(&name, &args, self.degrees);
        Ok(self.compute(t.col, &args, result))
    }
}
pub fn evaluate(src: &str, degrees: bool, ans: Option<f64>) -> Result<f64> {
    if src.encode_utf16().count() > 10000 {
        return Err(error(
            src,
            10001,
            "Ekspresi terlalu panjang (maksimum 10000 karakter)",
            false,
        ));
    }
    let (tok, next) = lex(src, 0)?;
    let mut e = Evaluator {
        src,
        tok,
        next,
        depth: 0,
        pending: None,
        degrees,
        ans,
    };
    let v = e.expr()?;
    if e.tok.kind != 3 {
        return Err(e.unexpected(None));
    }
    if let Some(err) = e.pending {
        return Err(err);
    }
    Ok(v)
}
