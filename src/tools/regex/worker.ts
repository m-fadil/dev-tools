import { parentPort, workerData } from 'node:worker_threads';
import { runMatch, type MatchJob, type WorkerReply } from './match.js';

let reply: WorkerReply;
try {
    reply = { ok: true, result: runMatch(workerData as MatchJob) };
} catch (error) {
    reply = { ok: false, error: error instanceof Error ? error.message : String(error) };
}
parentPort?.postMessage(reply);
