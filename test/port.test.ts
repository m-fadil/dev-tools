import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer, type Server } from 'node:net';
import { describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';

const CLI = fileURLToPath(new URL('../src/cli.js', import.meta.url));

/** run the CLI asynchronously (the test process hosts servers, so spawnSync would block them) */
const cli = (args: string[], timeoutMs?: number) =>
    new Promise<{ code: number | null; out: string; err: string }>((resolve, reject) => {
        const child = spawn(process.execPath, [CLI, ...args], {
            env: { PATH: process.env.PATH },
            stdio: ['ignore', 'pipe', 'pipe'],
            // This integration deadline bounds a real child process/network probe.
            timeout: timeoutMs,
            killSignal: 'SIGKILL',
        });
        let out = '';
        let err = '';
        child.stdout.setEncoding('utf8').on('data', (d: string) => (out += d));
        child.stderr.setEncoding('utf8').on('data', (d: string) => (err += d));
        child.on('error', reject);
        child.on('close', (code) => resolve({ code, out, err }));
    });

const listen = (port = 0) =>
    new Promise<{ server: Server; port: number }>((resolve, reject) => {
        const server = createServer();
        server.once('error', reject);
        server.listen(port, '127.0.0.1', () => resolve({ server, port: (server.address() as { port: number }).port }));
    });
const close = (server: Server) => new Promise<void>((resolve) => server.close(() => resolve()));

describe('port check', () => {
    test('open port exits 0, just-closed port exits 1', async () => {
        const { server, port } = await listen();
        try {
            const open = await cli(['port', 'check', String(port)]);
            assert.equal(open.code, 0);
            assert.equal(open.out.trim(), `127.0.0.1:${port} terbuka`);
        } finally {
            await close(server);
        }
        const closed = await cli(['port', 'check', String(port)]);
        assert.equal(closed.code, 1);
        assert.equal(closed.out.trim(), `127.0.0.1:${port} tertutup`);
    });

    test('port must be a strict integer 1-65535', async () => {
        for (const bad of ['0', '65536', '80x', '-1', '1.5', '']) {
            const r = await cli(['port', 'check', bad]);
            assert.equal(r.code, 2, `port "${bad}"`);
        }
        assert.equal((await cli(['port', 'check'])).code, 2);
        assert.equal((await cli(['port', 'check', '80', '--timeout', '0'])).code, 2);
        assert.equal((await cli(['port', 'check', '80', '--timeout', '60001'])).code, 2);
    });

    test('malformed hostname is an error, not a closed port', async () => {
        // Invalid DNS syntax fails locally, without relying on a resolver responding before timeout.
        const r = await cli(['port', 'check', '80', '--host', '[invalid]']);
        assert.equal(r.code, 1);
        assert.equal(r.out, '');
        assert.notEqual(r.err, '');
    });
});

describe('port free', () => {
    test('skips busy ports and prints the first bindable one', async () => {
        const { server, port } = await listen();
        try {
            if (port >= 65535) return;
            const r = await cli(['port', 'free', '--from', String(port), '--to', String(port + 1)]);
            // port + 1 could be taken by another process on a busy machine
            if (r.code === 0) assert.equal(r.out.trim(), String(port + 1));
            const busyOnly = await cli(['port', 'free', '--from', String(port), '--to', String(port)]);
            assert.equal(busyOnly.code, 1);
            assert.equal(busyOnly.out, '');
            assert.match(busyOnly.err, /Tidak ada port kosong/);
        } finally {
            await close(server);
        }
    });

    test('returns --from itself when it is free', async () => {
        const { server, port } = await listen();
        await close(server);
        const r = await cli(['port', 'free', '--from', String(port), '--to', String(port)]);
        assert.equal(r.code, 0);
        assert.equal(r.out.trim(), String(port));
    });

    test('invalid range and options are usage errors', async () => {
        assert.equal((await cli(['port', 'free', '--from', '10', '--to', '9'])).code, 2);
        assert.equal((await cli(['port', 'free', '--from', '0'])).code, 2);
        assert.equal((await cli(['port', 'free', '--to', '65536'])).code, 2);
        assert.equal((await cli(['port', 'free', 'extra'])).code, 2);
    });
});

describe('port subcommand', () => {
    test('missing or unknown subcommand is a usage error', async () => {
        assert.equal((await cli(['port'])).code, 2);
        const r = await cli(['port', 'scan']);
        assert.equal(r.code, 2);
        assert.match(r.err, /Subcommand tidak dikenal: scan/);
    });
});

describe('port --json', () => {
    test('check reports open and closed', async () => {
        const { server, port } = await listen();
        try {
            const open = await cli(['port', 'check', '--json', String(port)]);
            assert.equal(open.code, 0);
            assert.deepEqual(JSON.parse(open.out), { host: '127.0.0.1', port, status: 'open' });
        } finally {
            await close(server);
        }
        const closed = await cli(['port', 'check', '--json', String(port)]);
        assert.equal(closed.code, 1);
        assert.deepEqual(JSON.parse(closed.out), { host: '127.0.0.1', port, status: 'closed' });
    });

    test('free reports host and port', async () => {
        const r = await cli(['port', 'free', '--json', '--from', '20000', '--to', '65535']);
        assert.equal(r.code, 0);
        const doc = JSON.parse(r.out);
        assert.equal(doc.host, '127.0.0.1');
        assert.ok(doc.port >= 20000);
    });
});

describe('port check timeout and free failure', () => {
    test('check against a non-routable address ends as timeout or network error, never hangs', async (t) => {
        const started = Date.now();
        const r = await cli(['port', 'check', '--json', '--host', '10.255.255.1', '--timeout', '200', '80'], 5000);
        assert.ok(Date.now() - started < 5000);
        if (r.code === 1 && r.out === '' && /EPERM|EACCES|EAFNOSUPPORT|ENETDOWN/.test(r.err)) {
            t.skip('network stack unavailable in sandbox');
            return;
        }
        assert.equal(r.code, 1, r.err);
        if (r.out === '') {
            assert.match(r.err, /ENETUNREACH|EHOSTUNREACH|EADDRNOTAVAIL|ETIMEDOUT/);
        } else {
            assert.deepEqual(JSON.parse(r.out), { host: '10.255.255.1', port: 80, status: 'timeout' });
        }
    });

    test('free --json with no free port exits 1 and prints no JSON', async () => {
        const { server, port } = await listen();
        try {
            const r = await cli(['port', 'free', '--json', '--from', String(port), '--to', String(port)]);
            assert.equal(r.code, 1);
            assert.equal(r.out, '');
            assert.match(r.err, /Tidak ada port kosong/);
        } finally {
            await close(server);
        }
    });

    test('bad positional port message has no --', async () => {
        const r = await cli(['port', 'check', 'abc']);
        assert.equal(r.code, 2);
        assert.match(r.err, /Port harus bilangan bulat 1-65535: abc/);
        assert.doesNotMatch(r.err, /--port/);
    });
});
