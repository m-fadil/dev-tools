mod common;
use common::dt;

#[test]
fn top_level_help_lists_registered_commands() {
    let top = dt(&["--help"]);
    assert_eq!(top.code, 0);
    assert!(
        top.out
            .lines()
            .any(|l| l.starts_with("  syncgit      ") && l.len() > 15),
        "{}",
        top.out
    );
}

#[test]
fn unknown_command_exits_2() {
    let r = dt(&["nope"]);
    assert_eq!(r.code, 2);
    assert!(r.err.contains("Tool tidak dikenal: nope"));
    assert!(r.err.contains("Gunakan dt --help atau dt <tool> --help."));
}

#[test]
fn no_terminal_menu_fails_fast() {
    let r = dt(&[]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Menu memerlukan terminal"));
    assert!(r.out.contains("dt <tool> [options]"));
}

#[test]
fn empty_first_arg_acts_like_no_argument() {
    let r = dt(&[""]);
    assert_eq!(r.code, 1);
    assert!(r.err.contains("Menu memerlukan terminal"));
    assert!(r.out.contains("dt <tool> [options]"));
}
