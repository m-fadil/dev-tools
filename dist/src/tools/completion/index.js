import { parseToolArgs } from '../../io.js';
import { UsageError } from '../../errors.js';
import { tools } from '../index.js';
import { SHELLS, completionScript } from './scripts.js';
function help() {
    console.log(`dt completion <shell>

Cetak skrip auto-complete untuk nama tool dan alias (argumen pertama).
  shell   ${SHELLS.join(', ')}
  -h, --help  Tampilkan bantuan

Pasang:
  bash   echo 'source <(dt completion bash)' >> ~/.bashrc
  zsh    dt completion zsh > "\${fpath[1]}/_dt"   # lalu: autoload -U compinit && compinit
  fish   dt completion fish > ~/.config/fish/completions/dt.fish`);
}
async function run(args) {
    const { values, positionals } = parseToolArgs(args, { help: { type: 'boolean', short: 'h' } });
    if (values.help) {
        help();
        return;
    }
    const shell = positionals[0];
    if (positionals.length !== 1 || !SHELLS.includes(shell))
        throw new UsageError(`Tentukan shell: ${SHELLS.join(', ')}`);
    // `tools` is read at call time: the registry imports this module, so it is only complete after load.
    const entries = tools.flatMap((tool) => [tool.name, ...(tool.aliases ?? [])].map((name) => ({ name, description: tool.description })));
    process.stdout.write(completionScript(shell, entries));
}
export const completionTool = {
    name: 'completion',
    description: 'Cetak skrip auto-complete shell (bash, zsh, fish)',
    category: 'Utilitas',
    run,
};
//# sourceMappingURL=index.js.map