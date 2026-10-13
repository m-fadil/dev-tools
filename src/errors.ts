/** Invalid command line: unknown command/option or conflicting values. Exits 2, other failures exit 1. */
export class UsageError extends Error {
    override name = 'UsageError';
}
