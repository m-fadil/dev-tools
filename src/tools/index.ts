import type { Tool } from '../tool.js';
import { syncgit } from './syncgit/index.js';

// Registered tools share the menu, help, and command dispatch.
export const tools: readonly Tool[] = [syncgit];
