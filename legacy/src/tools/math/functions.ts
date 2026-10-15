import { DomainError } from './errors.js';

export const CONSTANTS: ReadonlyMap<string, number> = new Map([
    ['pi', Math.PI],
    ['e', Math.E],
    ['tau', 2 * Math.PI],
    ['inf', Infinity],
]);

export interface MathFunction {
    min: number;
    max: number;
    fn: (args: number[], degrees: boolean) => number;
}

const RAD = Math.PI / 180;

/** Exact sine at multiples of 30 degrees where the result is representable (0, ±0.5, ±1). */
const SIN_30 = [0, 0.5, undefined, 1, undefined, 0.5, 0, -0.5, undefined, -1, undefined, -0.5];

function sinDeg(x: number): number {
    const r = ((x % 360) + 360) % 360;
    return (r % 30 === 0 ? SIN_30[r / 30] : undefined) ?? Math.sin(x * RAD);
}

function cosDeg(x: number): number {
    const r = ((x % 360) + 360) % 360;
    return (r % 30 === 0 ? SIN_30[(r / 30 + 3) % 12] : undefined) ?? Math.cos(x * RAD);
}

function tanDeg(x: number): number {
    const r = ((x % 180) + 180) % 180;
    if (r === 90) throw new DomainError('tan tidak terdefinisi pada 90° + k·180°');
    if (r === 45) return 1;
    if (r === 135) return -1;
    return r === 0 ? 0 : Math.tan(x * RAD);
}

function requireInteger(name: string, ...values: number[]): void {
    if (!values.every(Number.isSafeInteger)) throw new DomainError(`${name} membutuhkan bilangan bulat (aman)`);
}

function gcd(a: number, b: number): number {
    a = Math.abs(a);
    b = Math.abs(b);
    while (b) [a, b] = [b, a % b];
    return a;
}

/** Half away from zero, unlike Math.round. */
function round(x: number): number {
    return Math.sign(x) * Math.round(Math.abs(x));
}

export function power(a: number, b: number): number {
    if (a === 0 && b < 0) throw new DomainError('Pembagian dengan nol (nol pangkat negatif)');
    return Math.pow(a, b);
}

export function remainder(a: number, b: number): number {
    if (b === 0) throw new DomainError('Modulo dengan nol');
    return a % b;
}

export function factorial(n: number): number {
    if (!Number.isInteger(n) || n < 0 || n > 170)
        throw new DomainError(`Faktorial hanya untuk bilangan bulat 0-170: ${n}`);
    let result = 1;
    for (let i = 2; i <= n; i++) result *= i;
    return result;
}

function positive(name: string, x: number): number {
    if (!(x > 0)) throw new DomainError(`${name} hanya terdefinisi untuk x > 0`);
    return x;
}

function unitRange(name: string, x: number): number {
    if (Math.abs(x) > 1) throw new DomainError(`${name} hanya terdefinisi untuk -1 <= x <= 1`);
    return x;
}

const fixed = (n: number, fn: (args: number[], degrees: boolean) => number): MathFunction => ({
    min: n,
    max: n,
    fn,
});
const unary = (fn: (x: number, degrees: boolean) => number) => fixed(1, ([x], d) => fn(x!, d));

export const FUNCTIONS: ReadonlyMap<string, MathFunction> = new Map<string, MathFunction>([
    [
        'sqrt',
        unary((x) => {
            if (x < 0) throw new DomainError('sqrt dari bilangan negatif');
            return Math.sqrt(x);
        }),
    ],
    ['cbrt', unary(Math.cbrt)],
    ['abs', unary(Math.abs)],
    ['sign', unary(Math.sign)],
    ['floor', unary(Math.floor)],
    ['ceil', unary(Math.ceil)],
    ['round', unary(round)],
    ['trunc', unary(Math.trunc)],
    ['exp', unary(Math.exp)],
    ['ln', unary((x) => Math.log(positive('ln', x)))],
    ['log', unary((x) => Math.log10(positive('log', x)))],
    ['log2', unary((x) => Math.log2(positive('log2', x)))],
    ['log10', unary((x) => Math.log10(positive('log10', x)))],
    ['sin', unary((x, d) => (d ? sinDeg(x) : Math.sin(x)))],
    ['cos', unary((x, d) => (d ? cosDeg(x) : Math.cos(x)))],
    ['tan', unary((x, d) => (d ? tanDeg(x) : Math.tan(x)))],
    ['asin', unary((x, d) => Math.asin(unitRange('asin', x)) * (d ? 1 / RAD : 1))],
    ['acos', unary((x, d) => Math.acos(unitRange('acos', x)) * (d ? 1 / RAD : 1))],
    ['atan', unary((x, d) => Math.atan(x) * (d ? 1 / RAD : 1))],
    ['atan2', fixed(2, ([y, x], d) => Math.atan2(y!, x!) * (d ? 1 / RAD : 1))],
    ['sinh', unary(Math.sinh)],
    ['cosh', unary(Math.cosh)],
    ['tanh', unary(Math.tanh)],
    ['min', { min: 1, max: Infinity, fn: (a) => Math.min(...a) }],
    ['max', { min: 1, max: Infinity, fn: (a) => Math.max(...a) }],
    ['pow', fixed(2, ([a, b]) => power(a!, b!))],
    ['hypot', { min: 1, max: Infinity, fn: (a) => Math.hypot(...a) }],
    ['mod', fixed(2, ([a, b]) => remainder(a!, b!))],
    [
        'gcd',
        fixed(2, ([a, b]) => {
            requireInteger('gcd', a!, b!);
            return gcd(a!, b!);
        }),
    ],
    [
        'lcm',
        fixed(2, ([a, b]) => {
            requireInteger('lcm', a!, b!);
            if (a === 0 || b === 0) return 0;
            const result = Math.abs(a! / gcd(a!, b!)) * Math.abs(b!);
            if (!Number.isSafeInteger(result)) throw new DomainError('Hasil lcm melebihi bilangan bulat aman');
            return result;
        }),
    ],
    ['deg', unary((x) => x / RAD)],
    ['rad', unary((x) => x * RAD)],
]);
