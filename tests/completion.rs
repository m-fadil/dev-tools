mod common;
use common::dt;

/// `(name, description)` for every command and alias, read from the top-level help.
fn registered() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in dt(&["--help"]).out.lines().filter(|l| l.starts_with("  ")) {
        let rest = line.trim_start();
        let (name, rest) = rest.split_once(' ').expect("name and description");
        let rest = rest.trim_start();
        let (description, aliases) = match rest.rsplit_once(" (alias: ") {
            Some((d, a)) => (d, a.trim_end_matches(')').split(", ").collect()),
            None => (rest, Vec::new()),
        };
        out.push((name.to_string(), description.to_string()));
        out.extend(aliases.into_iter().map(|a| (a.to_string(), description.to_string())));
    }
    out
}

#[test]
fn scripts_list_every_registered_command_alias_and_description() {
    let tools = registered();
    assert!(tools.len() > 38);
    let bash = dt(&["completion", "bash"]);
    let zsh = dt(&["completion", "zsh"]);
    let fish = dt(&["completion", "fish"]);
    for r in [&bash, &zsh, &fish] {
        assert_eq!(r.code, 0, "{}", r.err);
    }
    let words = bash
        .out
        .split("compgen -W '")
        .nth(1)
        .and_then(|s| s.split('\'').next())
        .expect("bash word list");
    let words: Vec<&str> = words.split(' ').collect();
    for (name, description) in &tools {
        assert!(words.contains(&name.as_str()), "bash lacks {name}");
        let zsh_entry = format!("'{name}:{}'", description.replace(':', "\\:"));
        assert!(zsh.out.lines().any(|l| l.trim() == zsh_entry), "zsh lacks {zsh_entry}");
        let fish_entry = format!("-a '{name}' -d '{description}'");
        assert!(
            fish.out.lines().any(|l| l.ends_with(&fish_entry)),
            "fish lacks {fish_entry}"
        );
    }
    assert_eq!(dt(&["completion", "powershell"]).code, 2);
    assert_eq!(dt(&["completion"]).code, 2);
}

#[test]
fn registers_only_dt_and_supports_zsh_autoload_and_source() {
    let bash = dt(&["completion", "bash"]).out;
    assert!(bash.starts_with("_dt() {\n"));
    assert!(bash.lines().any(|l| l == "complete -F _dt dt"));
    let zsh = dt(&["completion", "zsh"]).out;
    assert!(zsh.starts_with("#compdef dt\n_dt() {\n"));
    assert!(zsh.contains("if [[ $funcstack[1] == _dt ]]; then\n    _dt \"$@\""));
    assert!(zsh.lines().any(|l| l == "    compdef _dt dt"));
    assert!(
        dt(&["completion", "fish"])
            .out
            .lines()
            .all(|l| l.starts_with("complete -c dt "))
    );
}
