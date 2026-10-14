const SIGNIFICANT = 12;
function scientific(value, precision) {
    const text = value.toExponential(precision ?? SIGNIFICANT - 1);
    if (precision !== undefined)
        return text;
    return text.replace(/\.?0+e/, 'e');
}
/** Formats a finite or infinite result for display. */
export function formatNumber(value, { precision, scientific: sci }) {
    if (value === Infinity)
        return 'inf';
    if (value === -Infinity)
        return '-inf';
    let text;
    if (sci) {
        text = scientific(value, precision);
    }
    else if (precision !== undefined) {
        // toFixed falls back to exponent form from 1e21; such floats are always integers, so pad zeros.
        text =
            Math.abs(value) >= 1e21
                ? BigInt(value).toString() + (precision > 0 ? `.${'0'.repeat(precision)}` : '')
                : value.toFixed(precision);
    }
    else if (Number.isInteger(value) && Math.abs(value) < 1e21) {
        // Exact integers (even above 2^53) must not lose digits to the 12-digit rounding.
        text = BigInt(value).toString();
    }
    else {
        // Rounding hides float noise such as 0.1 + 0.2.
        const rounded = Number(value.toPrecision(SIGNIFICANT));
        text = Number.isInteger(rounded) && Math.abs(rounded) < 1e21 ? BigInt(rounded).toString() : String(rounded);
    }
    // No negative zero ("-0", "-0.00").
    return /^-0(\.0+)?$/.test(text) ? text.slice(1) : text;
}
//# sourceMappingURL=format.js.map