#!/usr/bin/env node
import { select } from '@inquirer/prompts';
import { UsageError } from './errors.js';
import { tools } from './tools/index.js';

function help(): void {
    console.log(`dt <tool> [options]

Tanpa argumen: menu interaktif. Ctrl+C: batalkan.

Tools:
${tools.map((tool) => `  ${tool.name.padEnd(12)} ${tool.description}`).join('\n')}

Gunakan dt <tool> --help untuk opsi tool.`);
}

async function main(args: string[]): Promise<void> {
    if (args[0] === '--help' || args[0] === '-h') {
        help();
        return;
    }
    let name = args[0];
    if (!name) {
        if (!process.stdin.isTTY || !process.stdout.isTTY) {
            help();
            throw new Error('Menu memerlukan terminal. Gunakan dt syncgit --repo <path> [options].');
        }
        name = await select({
            message: 'Pilih developer tool',
            choices: [
                ...tools.map((tool) => ({ name: tool.name, value: tool.name, description: tool.description })),
                { name: 'Keluar', value: '' },
            ],
        });
        if (!name) return;
    }
    const tool = tools.find((candidate) => candidate.name === name);
    if (!tool) throw new UsageError(`Tool tidak dikenal: ${name}`);
    await tool.run(args.slice(1));
}

try {
    await main(process.argv.slice(2));
} catch (error) {
    if (error instanceof Error && error.name === 'ExitPromptError') {
        console.error('Dibatalkan.');
        process.exitCode = 130;
    } else if (error instanceof UsageError) {
        console.error(`${error.message}\n\nGunakan dt --help atau dt <tool> --help.`);
        process.exitCode = 2;
    } else {
        console.error(error instanceof Error ? error.message : String(error));
        process.exitCode = 1;
    }
}
