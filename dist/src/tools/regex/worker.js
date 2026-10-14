import { parentPort, workerData } from 'node:worker_threads';
import { runMatch } from './match.js';
let reply;
try {
    reply = { ok: true, result: runMatch(workerData) };
}
catch (error) {
    reply = { ok: false, error: error instanceof Error ? error.message : String(error) };
}
parentPort?.postMessage(reply);
//# sourceMappingURL=worker.js.map