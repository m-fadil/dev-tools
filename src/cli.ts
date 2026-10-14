#!/usr/bin/env node
import { search } from '@inquirer/prompts';
import { UsageError } from './errors.js';
import type { Tool } from './tool.js';
import { findTool, tools, toolsByCategory } from './tools/index.js';

function toolLine(tool: Tool): string {
    const alias = tool.aliases?.length ? ` (alias: ${tool.aliases.join(', ')})` : '';
    return `  ${tool.name.padEnd(12)} ${tool.description}${alias}`;
}

function help(): void {
    const groups = toolsByCategory().map(([category, list]) => `${category}:\n${list.map(toolLine).join('\n')}`);
    console.log(`dt <tool> [options]

Tanpa argumen: menu interaktif dengan pencarian. Ctrl+C: batalkan.

${groups.join('\n\n')}

Gunakan dt <tool> --help untuk opsi tool.`);
}

function searchTools(term: string | undefined): { name: string; value: string; description: string }[] {
    const query = (term ?? '').trim().toLowerCase();
    const matches = tools.filter((tool) =>
        [tool.name, ...(tool.aliases ?? []), tool.description, tool.category].some((text) =>
            text.toLowerCase().includes(query),
        ),
    );
    const choices = matches.map((tool) => ({
        name: `${tool.category} › ${tool.name}`,
        value: tool.name,
        description: tool.description,
    }));
    return query ? choices : [...choices, { name: 'Keluar', value: '', description: '' }];
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
        name = await search({
            message: 'Pilih developer tool (ketik untuk mencari)',
            source: (term) => searchTools(term),
        });
        if (!name) return;
    }
    const tool = findTool(name);
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
