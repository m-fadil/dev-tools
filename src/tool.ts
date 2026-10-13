export interface Tool {
    readonly name: string;
    readonly description: string;
    run(args: string[]): Promise<void>;
}
