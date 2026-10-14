const DEFAULT_PORTS = {
    'http:': 80,
    'https:': 443,
    'ftp:': 21,
    'ws:': 80,
    'wss:': 443,
};
/** Parses an absolute URL (no base); throws a user-facing Error when invalid. */
export function parseUrl(text, showPassword) {
    let url;
    try {
        url = new URL(text);
    }
    catch {
        const hint = /^[a-z][a-z0-9+.-]*:/i.test(text) ? '' : ' (tambahkan skema, mis. https://)';
        // Never echo credentials. Conservatively mask through the last "@" in invalid authority-style or
        // scheme-less input, even when a malformed password contains "/", "?", or "#".
        // Opaque URLs like mailto:a@b have no userinfo and stay as is.
        const shown = text.trim().replace(/^((?:[a-z][a-z0-9+.-]*:)?\/\/|(?![a-z][a-z0-9+.-]*:))[\s\S]*@/i, '$1***@');
        // The original TypeError carries the raw input in `.input`, so it must not become the cause.
        throw new Error(`URL tidak valid: ${shown}${hint}`, { cause: new TypeError('Invalid URL') });
    }
    return {
        protocol: url.protocol,
        username: url.username,
        password: url.password && !showPassword ? '***' : url.password,
        host: url.host,
        hostname: url.hostname,
        port: url.port,
        pathname: url.pathname,
        search: url.search,
        hash: url.hash,
        origin: url.origin,
        query: [...url.searchParams.entries()],
    };
}
/** Explicit port, or "(default n)" for known schemes; undefined when neither applies. */
export function portLabel(parsed) {
    if (parsed.port)
        return parsed.port;
    const fallback = DEFAULT_PORTS[parsed.protocol];
    return fallback === undefined ? undefined : `(default ${fallback})`;
}
export function formatUrl(parsed) {
    const fields = [
        ['protocol', parsed.protocol],
        ['username', parsed.username],
        ['password', parsed.password],
        ['host', parsed.host],
        ['hostname', parsed.hostname],
        ['port', portLabel(parsed)],
        ['pathname', parsed.pathname],
        ['search', parsed.search],
        ['hash', parsed.hash],
        // Opaque origins serialize as "null"; that is noise, not information.
        ['origin', parsed.origin === 'null' ? undefined : parsed.origin],
    ];
    const shown = fields.filter((f) => !!f[1]);
    const width = Math.max(...shown.map(([label]) => label.length)) + 1;
    const lines = shown.map(([label, value]) => `${(label + ':').padEnd(width + 1)}${value}`);
    if (parsed.query.length) {
        const keyWidth = Math.max(...parsed.query.map(([key]) => key.length));
        lines.push('', 'Query:', ...parsed.query.map(([key, value]) => `  ${key.padEnd(keyWidth)} = ${value}`));
    }
    return lines.join('\n');
}
//# sourceMappingURL=url.js.map