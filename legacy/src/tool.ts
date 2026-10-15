export interface Tool {
    readonly name: string;
    /** Alternative command names; must not collide with any tool name or alias. */
    readonly aliases?: readonly string[];
    readonly description: string;
    /** Group heading in help and the interactive menu. */
    readonly category: string;
    run(args: string[]): Promise<void>;
}
