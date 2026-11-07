//! The `sau` command (Cosmósis spec, section 7): project commands without Sau running,
//! headless commands that start Sau, and actions on the open app through its gateway.
//!
//! Built so far: `sau check`, up to the layout stage (`cosmosis::layout`); reading the
//! manifest and the Null files waits for Null's parser (Sau tasks 1a-03 and 1a-11). The other
//! commands are listed by `sau --help` and say they are not built yet.
//!
//! Exit status: 0 when the command succeeded, 1 when it found errors, 2 on wrong usage or a
//! command that is not built yet.

use std::path::Path;
use std::process::ExitCode;

use cosmosis::diagnostic::{Diagnostic, nearest};
use cosmosis::layout;

/// Every command in the spec: name, what it does, and whether it is built.
const COMMANDS: [(&str, &str, bool); 15] = [
    ("new", "Start a project from a template", false),
    ("open", "Open a project in Sau", false),
    (
        "dev",
        "Watch a project, check every save and apply it to Sau",
        false,
    ),
    ("watch", "Print a live value each time it changes", false),
    (
        "check",
        "Check a project's layout and files, with diagnostics",
        true,
    ),
    ("fmt", "Print files in canonical form", false),
    ("convert", "Convert a .blend to a project and back", false),
    (
        "import",
        "Import any format Blender reads into the project",
        false,
    ),
    (
        "export",
        "Export the project to any format Blender writes",
        false,
    ),
    (
        "render",
        "Render stills, animations and passes, headless",
        false,
    ),
    (
        "bake",
        "Bake simulations, lighting and textures into media/",
        false,
    ),
    ("test", "Run the project's checks", false),
    ("diff", "Compare two commits as text and as renders", false),
    ("lift", "Lift geometry or images to higher levels", false),
    ("lsp", "Start the language server", false),
];

const USAGE: u8 = 2;

fn help() -> String {
    let mut s = format!(
        "sau {}: the command of Cosmósis, Sau's project framework\n\n\
         Usage: sau <command> [arguments]\n\nCommands:\n",
        env!("CARGO_PKG_VERSION")
    );
    for (name, what, built) in COMMANDS {
        let mark = if built { "" } else { " (not built yet)" };
        s.push_str(&format!("  {name:<9}{what}{mark}\n"));
    }
    s.push_str(
        "\nOptions:\n  -h, --help     Print this help\n  -V, --version  Print the version\n\n\
         `sau <command> --help` describes one command.\n",
    );
    s
}

const CHECK_HELP: &str = "Usage: sau check [path]

Checks the project holding <path>, a folder or a file in it (default: the current folder).
The project is the nearest folder, from <path> upwards, holding cosmosis.noll.

Checked now: the layout (cosmosis.noll and the root's other Null files are files; every .noll
file is UTF-8 text; media/ and data/ files are named by their SHA-256). Reading the manifest
and the Null files comes with Null's parser.

Exit status: 0 when no errors were found, 1 when some were, 2 on wrong usage.
";

fn usage_error(message: &str) -> ExitCode {
    eprintln!("sau: {message}\nRun `sau --help` for the commands.");
    ExitCode::from(USAGE)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(command) = args.first() else {
        eprint!("{}", help());
        return ExitCode::from(USAGE);
    };
    match command.as_str() {
        "-h" | "--help" | "help" => {
            print!("{}", help());
            ExitCode::SUCCESS
        }
        "-V" | "--version" => {
            println!("sau {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "check" => check(&args[1..]),
        name => match COMMANDS.iter().find(|c| c.0 == name) {
            Some(_) => {
                eprintln!("sau: `{name}` is not built yet (Cosmósis spec, section 7)");
                ExitCode::from(USAGE)
            }
            None => {
                let names: Vec<&str> = COMMANDS.iter().map(|c| c.0).collect();
                match nearest(name, &names) {
                    Some(s) => usage_error(&format!("no command `{name}`; did you mean `{s}`?")),
                    None => usage_error(&format!("no command `{name}`")),
                }
            }
        },
    }
}

/// `sau check [path]`.
fn check(args: &[String]) -> ExitCode {
    let mut path = None;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{CHECK_HELP}");
                return ExitCode::SUCCESS;
            }
            flag if flag.starts_with('-') => {
                return usage_error(&format!("`sau check` has no option `{flag}`"));
            }
            p if path.is_none() => path = Some(p),
            p => return usage_error(&format!("`sau check` takes one path; `{p}` is a second")),
        }
    }
    let root = match layout::find_root(Path::new(path.unwrap_or("."))) {
        Ok(root) => root,
        Err(d) => {
            report(&[d]);
            return ExitCode::FAILURE;
        }
    };
    let found = layout::check(&root, None);
    report(&found.diagnostics);
    let errors = found.errors();
    if errors > 0 {
        let s = if errors == 1 { "" } else { "s" };
        eprintln!("sau check: {errors} error{s} in {}", root.display());
        return ExitCode::FAILURE;
    }
    report(&[Diagnostic::note(
        "cosmosis-not-parsed",
        "the manifest and the Null files are not read yet: that needs Null's parser (Sau tasks 1a-03 and 1a-11)",
    )]);
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    let (n, c) = (found.sources.len(), found.content_files);
    println!(
        "sau check: {}: the layout is valid ({n} Null file{}, {c} content file{})",
        root.display(),
        plural(n),
        plural(c),
    );
    ExitCode::SUCCESS
}

fn report(diagnostics: &[Diagnostic]) {
    for d in diagnostics {
        eprintln!("{d}\n");
    }
}
