import { input } from '@inquirer/prompts';
import { readFile } from 'node:fs/promises';
import { parseArgs, type ParseArgsOptionsConfig } from 'node:util';
import { UsageError } from './errors.js';

/** parseArgs with positionals allowed; malformed options become UsageError (exit 2). */
export function parseToolArgs<const O extends ParseArgsOptionsConfig>(args: string[], options: O) {
    try {
        return parseArgs({ args, allowPositionals: true, options });
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code?.startsWith('ERR_PARSE_ARGS')) {
            throw new UsageError((error as Error).message, { cause: error });
        }
        throw error;
    }
}

/** Validates an option value against an allowed list; undefined yields the fallback. */
export function choice<T extends string>(
    value: string | undefined,
    allowed: readonly T[],
    fallback: T,
    option: string,
): T {
    if (value === undefined) return fallback;
    if ((allowed as readonly string[]).includes(value)) return value as T;
    throw new UsageError(`Nilai --${option} tidak valid: ${value}. Pilihan: ${allowed.join(', ')}`);
}

/** Strict positive integer within [min, max]. */
export function integer(value: string | undefined, fallback: number, min: number, max: number, option: string): number {
    if (value === undefined) return fallback;
    const n = /^\d+$/.test(value) ? Number(value) : NaN;
    if (!Number.isSafeInteger(n) || n < min || n > max) {
        throw new UsageError(`Nilai --${option} harus bilangan bulat ${min}-${max}: ${value}`);
    }
    return n;
}

async function readStdin(): Promise<Buffer> {
    const chunks: Buffer[] = [];
    for await (const chunk of process.stdin) chunks.push(chunk as Buffer);
    return Buffer.concat(chunks);
}

/**
 * Input bytes from positional text, --file, piped stdin, or (terminal only) a prompt.
 * Bytes are used verbatim: piped input keeps its trailing newline, like sha256sum.
 */
export async function readInput(positionals: string[], file: string | undefined, message: string): Promise<Buffer> {
    if (file !== undefined && positionals.length) throw new UsageError('Gunakan teks atau --file, bukan keduanya.');
    if (file !== undefined) return readFile(file);
    if (positionals.length) return Buffer.from(positionals.join(' '), 'utf8');
    if (!process.stdin.isTTY) return readStdin();
    // Prompt on stderr so stdout stays clean for pipes.
    return Buffer.from(await input({ message }, { output: process.stderr }), 'utf8');
}

/** Writes result bytes verbatim; on a terminal ensures the prompt starts on a new line. */
export function writeOutput(data: Buffer | string): void {
    const buffer = typeof data === 'string' ? Buffer.from(data, 'utf8') : data;
    process.stdout.write(buffer);
    if (process.stdout.isTTY && buffer.at(-1) !== 0x0a) process.stdout.write('\n');
}

/** Machine-readable result for `--json`: one pretty-printed JSON document on stdout. */
export function printJson(value: unknown): void {
    console.log(JSON.stringify(value, null, 2));
}
