use crate::numfmt::js_number;
pub fn arity(n: &str) -> Option<(usize, usize)> {
    Some(match n {
        "min" | "max" | "hypot" => (1, usize::MAX),
        "atan2" | "pow" | "mod" | "gcd" | "lcm" => (2, 2),
        "sqrt" | "cbrt" | "abs" | "sign" | "floor" | "ceil" | "round" | "trunc" | "exp" | "ln" | "log" | "log2"
        | "log10" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "deg" | "rad" => {
            (1, 1)
        }
        _ => return None,
    })
}
pub fn power(a: f64, b: f64) -> Result<f64, String> {
    if a == 0. && b < 0. {
        Err("Pembagian dengan nol (nol pangkat negatif)".into())
    } else if a.abs() == 1. && b.is_infinite() {
        Ok(f64::NAN)
    } else {
        Ok(a.powf(b))
    }
}
pub fn remainder(a: f64, b: f64) -> Result<f64, String> {
    if b == 0. {
        Err("Modulo dengan nol".into())
    } else {
        Ok(a % b)
    }
}
pub fn factorial(n: f64) -> Result<f64, String> {
    if n.fract() != 0. || !(0. ..=170.).contains(&n) {
        return Err(format!("Faktorial hanya untuk bilangan bulat 0-170: {}", js_number(n)));
    }
    let mut r = 1.;
    for i in 2..=n as u32 {
        r *= f64::from(i);
    }
    Ok(r)
}
fn gcd(mut a: f64, mut b: f64) -> f64 {
    a = a.abs();
    b = b.abs();
    while b != 0. {
        (a, b) = (b, a % b);
    }
    a
}
pub fn call(n: &str, a: &[f64], degrees: bool) -> Result<f64, String> {
    let x = a[0];
    let rad = std::f64::consts::PI / 180.;
    let out = match n {
        "sqrt" => {
            if x < 0. {
                return Err("sqrt dari bilangan negatif".into());
            }
            x.sqrt()
        }
        "cbrt" => x.cbrt(),
        "abs" => x.abs(),
        "sign" => {
            if x == 0. {
                x
            } else {
                x.signum()
            }
        }
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        "round" => {
            if x.fract() == 0.0 {
                x
            } else {
                {
                    let magnitude = x.abs();
                    let floor = magnitude.floor();
                    x.signum() * if magnitude - floor >= 0.5 { floor + 1.0 } else { floor }
                }
            }
        }
        "trunc" => x.trunc(),
        "exp" => x.exp(),
        "ln" | "log" | "log10" | "log2" => {
            if x <= 0. {
                return Err(format!("{n} hanya terdefinisi untuk x > 0"));
            }
            match n {
                "ln" => x.ln(),
                "log2" => x.log2(),
                _ => x.log10(),
            }
        }
        "sin" | "cos" => {
            let r = ((x % 360.) + 360.) % 360.;
            let index = if n == "sin" { r / 30. } else { (r / 30. + 3.) % 12. };
            let exact = if degrees && r % 30. == 0. {
                match index as usize {
                    0 | 6 => Some(0.),
                    1 | 5 => Some(0.5),
                    3 => Some(1.),
                    7 | 11 => Some(-0.5),
                    9 => Some(-1.),
                    _ => None,
                }
            } else {
                None
            };
            exact.unwrap_or_else(|| {
                let v = if degrees { x * rad } else { x };
                if n == "sin" { v.sin() } else { v.cos() }
            })
        }
        "tan" => {
            let r = ((x % 180.) + 180.) % 180.;
            if degrees && r == 90. {
                return Err("tan tidak terdefinisi pada 90° + k·180°".into());
            }
            if degrees && r == 45. {
                1.
            } else if degrees && r == 135. {
                -1.
            } else if degrees && r == 0. {
                0.
            } else {
                (if degrees { x * rad } else { x }).tan()
            }
        }
        "asin" | "acos" => {
            if x.abs() > 1. {
                return Err(format!("{n} hanya terdefinisi untuk -1 <= x <= 1"));
            }
            (if n == "asin" { x.asin() } else { x.acos() }) * if degrees { 1. / rad } else { 1. }
        }
        "atan" => x.atan() * if degrees { 1. / rad } else { 1. },
        "atan2" => x.atan2(a[1]) * if degrees { 1. / rad } else { 1. },
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "min" => a.iter().copied().fold(f64::INFINITY, |x, y| {
            if x == 0. && y == 0. {
                if x.is_sign_negative() || y.is_sign_negative() {
                    -0.
                } else {
                    0.
                }
            } else {
                x.min(y)
            }
        }),
        "max" => a.iter().copied().fold(f64::NEG_INFINITY, |x, y| {
            if x == 0. && y == 0. {
                if x.is_sign_positive() || y.is_sign_positive() {
                    0.
                } else {
                    -0.
                }
            } else {
                x.max(y)
            }
        }),
        "hypot" => {
            let max = a.iter().map(|x| x.abs()).fold(0., f64::max);
            if max == 0. || max.is_infinite() {
                max
            } else {
                let mut sum = 0.;
                let mut compensation = 0.;
                for v in a {
                    let v = v.abs() / max;
                    let square = v * v;
                    let adjusted = square - compensation;
                    let next = sum + adjusted;
                    compensation = (next - sum) - adjusted;
                    sum = next;
                }
                sum.sqrt() * max
            }
        }
        "pow" => return power(x, a[1]),
        "mod" => return remainder(x, a[1]),
        "gcd" | "lcm" => {
            if a.iter()
                .any(|v| !v.is_finite() || v.fract() != 0. || v.abs() > 9007199254740991.)
            {
                return Err(format!("{n} membutuhkan bilangan bulat (aman)"));
            }
            if n == "gcd" {
                gcd(x, a[1])
            } else if x == 0. || a[1] == 0. {
                0.
            } else {
                let r = (x / gcd(x, a[1])).abs() * a[1].abs();
                if r > 9007199254740991. {
                    return Err("Hasil lcm melebihi bilangan bulat aman".into());
                }
                r
            }
        }
        "deg" => x / rad,
        "rad" => x * rad,
        _ => return Err(format!("Identifier tidak dikenal: {n}")),
    };
    Ok(out)
}
