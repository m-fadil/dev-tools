import { domainError, DomainError, syntaxError } from './errors.js';
import { CONSTANTS, factorial, FUNCTIONS, power, remainder } from './functions.js';
import { lex } from './lexer.js';
export const MAX_LENGTH = 10_000;
export const MAX_DEPTH = 200;
/**
 * Evaluates while parsing (recursive descent), so no AST and no dynamic code execution.
 *
 *   expr    = term (('+' | '-') term)*
 *   term    = unary (('*' | '/' | '%') unary)*
 *   unary   = ('-' | '+') unary | power
 *   power   = postfix (('^' | '**') exponent)?      right-associative
 *   exponent= ('-' | '+') exponent | power          so 2^-1 works and 2^-3^2 = 2^(-(3^2))
 *   postfix = primary '!'*
 *   primary = number | constant | function '(' [expr (',' expr)*] ')' | '(' expr ')'
 */
class Evaluator {
    src;
    opts;
    tok;
    next;
    depth = 0;
    pending;
    constructor(src, opts) {
        this.src = src;
        this.opts = opts;
        ({ token: this.tok, next: this.next } = lex(src, 0));
    }
    run() {
        const value = this.expr();
        if (this.tok.kind !== 'end')
            throw this.unexpected();
        if (this.pending)
            throw this.pending;
        return value;
    }
    advance() {
        ({ token: this.tok, next: this.next } = lex(this.src, this.next));
    }
    isOp(...ops) {
        return this.tok.kind === 'op' && ops.includes(this.tok.text);
    }
    enter() {
        if (++this.depth > MAX_DEPTH) {
            throw syntaxError(this.src, this.tok.col, `Ekspresi terlalu dalam (maksimum ${MAX_DEPTH} tingkat)`);
        }
    }
    unexpected(expected) {
        const { kind, text, col } = this.tok;
        if (kind === 'end') {
            return syntaxError(this.src, col, `Ekspresi berakhir tiba-tiba${expected ? `, diharapkan ${expected}` : ''}`);
        }
        if (kind === 'num' || kind === 'id' || text === '(') {
            return syntaxError(this.src, col, `Perkalian implisit tidak didukung; tulis '*' secara eksplisit (ditemukan '${text}')`);
        }
        return syntaxError(this.src, col, `Token tidak terduga '${text}'${expected ? `, diharapkan ${expected}` : ''}`);
    }
    /**
     * Runs a computation, attaching the column to domain failures and rejecting NaN/overflow.
     * The first failure is only recorded (parsing continues with 0) so a later syntax error still wins.
     */
    compute(col, inputs, fn) {
        if (this.pending)
            return 0;
        let result;
        try {
            result = fn();
        }
        catch (error) {
            if (error instanceof DomainError)
                return this.fail(domainError(this.src, col, error.message));
            throw error;
        }
        if (Number.isNaN(result))
            return this.fail(domainError(this.src, col, 'Hasil tidak terdefinisi (NaN)'));
        if (!Number.isFinite(result) && inputs.every(Number.isFinite)) {
            return this.fail(domainError(this.src, col, 'Hasil melampaui batas bilangan (tak berhingga)'));
        }
        return result;
    }
    fail(error) {
        this.pending ??= error;
        return 0;
    }
    expr() {
        let left = this.term();
        while (this.isOp('+', '-')) {
            const { text, col } = this.tok;
            this.advance();
            const right = this.term();
            const l = left;
            left = this.compute(col, [l, right], () => (text === '+' ? l + right : l - right));
        }
        return left;
    }
    term() {
        let left = this.unary();
        while (this.isOp('*', '/', '%')) {
            const { text, col } = this.tok;
            this.advance();
            const right = this.unary();
            const l = left;
            left = this.compute(col, [l, right], () => {
                if (text === '*')
                    return l * right;
                if (text === '%')
                    return remainder(l, right);
                if (right === 0)
                    throw new DomainError('Pembagian dengan nol');
                return l / right;
            });
        }
        return left;
    }
    unary() {
        if (!this.isOp('-', '+'))
            return this.power();
        this.enter();
        const negate = this.tok.text === '-';
        this.advance();
        const operand = this.unary();
        this.depth--;
        return negate ? -operand : operand;
    }
    power() {
        const base = this.postfix();
        if (!this.isOp('^', '**'))
            return base;
        const { col } = this.tok;
        this.advance();
        this.enter();
        const exponent = this.exponent();
        this.depth--;
        return this.compute(col, [base, exponent], () => power(base, exponent));
    }
    exponent() {
        if (!this.isOp('-', '+'))
            return this.power();
        this.enter();
        const negate = this.tok.text === '-';
        this.advance();
        const operand = this.exponent();
        this.depth--;
        return negate ? -operand : operand;
    }
    postfix() {
        let value = this.primary();
        while (this.isOp('!')) {
            const { col } = this.tok;
            const operand = value;
            value = this.compute(col, [operand], () => factorial(operand));
            this.advance();
        }
        return value;
    }
    primary() {
        const { kind, text, col } = this.tok;
        if (kind === 'num') {
            const { value } = this.tok;
            this.advance();
            return value;
        }
        if (kind === 'id')
            return this.identifier(text.toLowerCase(), text, col);
        if (this.isOp('(')) {
            this.enter();
            this.advance();
            const value = this.expr();
            this.depth--;
            if (!this.isOp(')'))
                throw this.unexpected("')'");
            this.advance();
            return value;
        }
        throw this.unexpected("angka, konstanta, fungsi, atau '('");
    }
    identifier(name, text, col) {
        // Names are checked before consuming, so unknown ones are reported before whatever follows them.
        if (name === 'ans') {
            if (this.opts.ans === undefined) {
                this.fail(domainError(this.src, col, "'ans' belum ada: belum ada hasil sebelumnya"));
                this.advance();
                return 0;
            }
            this.advance();
            return this.opts.ans;
        }
        const constant = CONSTANTS.get(name);
        if (constant !== undefined) {
            this.advance();
            return constant;
        }
        const func = FUNCTIONS.get(name);
        if (!func)
            throw syntaxError(this.src, col, `Identifier tidak dikenal: ${text}`);
        this.advance();
        if (!this.isOp('('))
            throw syntaxError(this.src, this.tok.col, `'(' diharapkan setelah fungsi ${name}`);
        this.enter();
        this.advance();
        const args = [];
        if (!this.isOp(')')) {
            for (;;) {
                args.push(this.expr());
                if (!this.isOp(','))
                    break;
                this.advance();
            }
        }
        if (!this.isOp(')'))
            throw this.unexpected("',' atau ')'");
        this.depth--;
        this.advance();
        if (args.length < func.min || args.length > func.max) {
            const range = func.min === func.max
                ? `${func.min}`
                : func.max === Infinity
                    ? `minimal ${func.min}`
                    : `${func.min}-${func.max}`;
            throw syntaxError(this.src, col, `Fungsi ${name} membutuhkan ${range} argumen, diberikan ${args.length}`);
        }
        return this.compute(col, args, () => func.fn(args, this.opts.degrees));
    }
}
/** Evaluates one expression. Syntax errors are UsageError (exit 2); domain errors plain Error (exit 1). */
export function evaluate(src, opts) {
    if (src.length > MAX_LENGTH) {
        throw syntaxError(src, MAX_LENGTH + 1, `Ekspresi terlalu panjang (maksimum ${MAX_LENGTH} karakter)`);
    }
    return new Evaluator(src, opts).run();
}
//# sourceMappingURL=evaluate.js.map