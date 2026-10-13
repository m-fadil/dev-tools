import type { Tool } from '../tool.js';
import { copygit } from './copygit/index.js';

// Registered tools share the menu, help, and command dispatch.
export const tools: readonly Tool[] = [copygit];
