import { Worker } from 'node:worker_threads';
import type { MatchJob, MatchResult, WorkerReply } from './match.js';

export const TIMEOUT_MESSAGE = 'regex timeout (kemungkinan catastrophic backtracking)';
export const MEMORY_MESSAGE = 'regex melebihi batas memori';

/** Runs the match in a worker thread that is terminated after `timeoutMs`, so a runaway pattern cannot hang the CLI. */
export function matchInWorker(job: MatchJob, timeoutMs: number): Promise<MatchResult> {
    return new Promise((resolve, reject) => {
        const worker = new Worker(new URL('./worker.js', import.meta.url), {
            workerData: job,
            resourceLimits: { maxOldGenerationSizeMb: 256 },
        });
        let settled = false;
        const settle = (finish: () => void): void => {
            if (settled) return;
            settled = true;
            clearTimeout(timer);
            void worker.terminate();
            finish();
        };
        const timer = setTimeout(() => settle(() => reject(new Error(TIMEOUT_MESSAGE))), timeoutMs);
        worker.once('message', (reply: WorkerReply) =>
            settle(() => (reply.ok ? resolve(reply.result) : reject(new Error(reply.error)))),
        );
        worker.once('error', (error: NodeJS.ErrnoException) =>
            settle(() => reject(error.code === 'ERR_WORKER_OUT_OF_MEMORY' ? new Error(MEMORY_MESSAGE) : error)),
        );
        worker.once('exit', (code) =>
            settle(() => reject(new Error(`Worker regex berhenti tak terduga (kode ${code}).`))),
        );
    });
}
