import { randomInt as cryptoRandomInt } from 'node:crypto';
import { UsageError } from '../../errors.js';
export const ONLY_SETS = ['lower', 'upper', 'digits', 'alnum', 'hex'];
export const SYMBOLS = '!@#$%^&*()-_=+[]{};:,.?';
const LOWER = 'abcdefghijklmnopqrstuvwxyz';
const UPPER = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
const DIGITS = '0123456789';
const AMBIGUOUS = '0O1lI|';
/** Character classes of the active set; one character of each is guaranteed in every password. */
export function characterClasses({ symbols, only, unambiguous }) {
    if (only && symbols)
        throw new UsageError('--only tidak bisa dipakai bersama --symbols.');
    let classes;
    switch (only) {
        case 'lower':
            classes = [LOWER];
            break;
        case 'upper':
            classes = [UPPER];
            break;
        case 'digits':
            classes = [DIGITS];
            break;
        case 'hex':
            // A single class: hex has no letter/digit guarantee.
            classes = [DIGITS + 'abcdef'];
            break;
        default:
            classes = [LOWER, UPPER, DIGITS];
            if (symbols)
                classes.push(SYMBOLS);
    }
    if (!unambiguous)
        return classes;
    return classes.map((set) => [...set].filter((c) => !AMBIGUOUS.includes(c)).join(''));
}
/** Upper estimate in bits: length * log2(alphabet size); ignores the one-per-class guarantee. */
export function entropyBits(classes, length) {
    const size = classes.reduce((sum, set) => sum + set.length, 0);
    return length * Math.log2(size);
}
/**
 * One required character per class, the rest from the whole alphabet, then a Fisher-Yates shuffle
 * so the required characters do not sit at fixed positions. Every pick is an unbiased randomInt.
 */
export function generatePassword(classes, length, randomInt = cryptoRandomInt) {
    if (length < classes.length) {
        throw new UsageError(`--length minimal ${classes.length} agar tiap jenis karakter terwakili.`);
    }
    const alphabet = classes.join('');
    const chars = classes.map((set) => set[randomInt(set.length)]);
    while (chars.length < length)
        chars.push(alphabet[randomInt(alphabet.length)]);
    for (let i = chars.length - 1; i > 0; i--) {
        const j = randomInt(i + 1);
        [chars[i], chars[j]] = [chars[j], chars[i]];
    }
    return chars.join('');
}
//# sourceMappingURL=generate.js.map