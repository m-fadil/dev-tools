import { connect, createServer } from 'node:net';
/** TCP connect attempt; the socket is always destroyed. Non-refusal socket errors reject. */
export function checkPort(host, port, timeoutMs) {
    return new Promise((resolve, reject) => {
        const socket = connect({ host, port });
        const finish = (result) => {
            clearTimeout(timer);
            socket.destroy();
            if (result instanceof Error)
                reject(result);
            else
                resolve(result);
        };
        const timer = setTimeout(() => finish('timeout'), timeoutMs);
        socket.once('connect', () => finish('terbuka'));
        socket.once('error', (error) => {
            finish(error.code === 'ECONNREFUSED' ? 'tertutup' : error);
        });
    });
}
/** True if the port can be bound (listen, then close); address-in-use and permission denial mean false. */
export function canBind(host, port) {
    return new Promise((resolve, reject) => {
        const server = createServer();
        server.once('error', (error) => {
            if (error.code === 'EADDRINUSE' || error.code === 'EACCES')
                resolve(false);
            else
                reject(error);
        });
        server.listen({ host, port, exclusive: true }, () => {
            server.close((error) => (error ? reject(error) : resolve(true)));
        });
    });
}
/** First bindable port in [from, to], or undefined. */
export async function findFreePort(host, from, to) {
    for (let port = from; port <= to; port++) {
        if (await canBind(host, port))
            return port;
    }
    return undefined;
}
//# sourceMappingURL=probe.js.map