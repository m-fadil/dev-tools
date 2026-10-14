import type { Tool } from '../tool.js';
import { asciiTool } from './ascii/index.js';
import { bytesTool } from './bytes/index.js';
import { caseTool } from './case/index.js';
import { checksumTool } from './checksum/index.js';
import { cidrTool } from './cidr/index.js';
import { colorTool } from './color/index.js';
import { completionTool } from './completion/index.js';
import { cronTool } from './cron/index.js';
import { csvTool } from './csv/index.js';
import { decodeTool } from './decode/index.js';
import { diffTool } from './diff/index.js';
import { encodeTool } from './encode/index.js';
import { encryptTool } from './encrypt/index.js';
import { envTool } from './env/index.js';
import { fakeTool } from './fake/index.js';
import { hashTool } from './hash/index.js';
import { imageTool } from './image/index.js';
import { jsonTool } from './json/index.js';
import { jwtTool } from './jwt/index.js';
import { loremTool } from './lorem/index.js';
import { macTool } from './mac/index.js';
import { mathTool } from './math/index.js';
import { numberTool } from './number/index.js';
import { passwordTool } from './password/index.js';
import { portTool } from './port/index.js';
import { regexTool } from './regex/index.js';
import { semverTool } from './semver/index.js';
import { syncgit } from './syncgit/index.js';
import { textTool } from './text/index.js';
import { timeTool } from './time/index.js';
import { urlTool } from './url/index.js';
import { uuid } from './uuid/index.js';

// Registered tools share the menu, help, and command dispatch.
export const tools: readonly Tool[] = [
    syncgit,
    hashTool,
    checksumTool,
    encryptTool,
    encodeTool,
    decodeTool,
    uuid,
    passwordTool,
    jwtTool,
    jsonTool,
    timeTool,
    cronTool,
    urlTool,
    diffTool,
    envTool,
    portTool,
    asciiTool,
    bytesTool,
    caseTool,
    cidrTool,
    colorTool,
    completionTool,
    csvTool,
    fakeTool,
    imageTool,
    loremTool,
    macTool,
    mathTool,
    numberTool,
    regexTool,
    semverTool,
    textTool,
];

/** Resolves a command name or alias. */
export function findTool(name: string): Tool | undefined {
    return tools.find((tool) => tool.name === name || tool.aliases?.includes(name));
}

/** Tools grouped by category, categories in first-appearance order, tools sorted by name. */
export function toolsByCategory(): [string, Tool[]][] {
    const groups = new Map<string, Tool[]>();
    for (const tool of tools) groups.set(tool.category, [...(groups.get(tool.category) ?? []), tool]);
    return [...groups].map(([category, list]) => [category, list.sort((a, b) => a.name.localeCompare(b.name))]);
}
