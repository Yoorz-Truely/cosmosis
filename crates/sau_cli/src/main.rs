//! The `sau` command (Cosmósis spec, section 7): project commands without Sau running,
//! headless commands that start Sau, and actions on the open app through its gateway.
//!
//! Built so far: `sau check`, which reads the manifest, checks the layout, parses every Null
//! file with Null's parser and checks them against Blender's schema with Null's checker (Sau
//! tasks C-01, 1a-11 and 1a-15). The other commands are listed by `sau --help` and say they are
//! not built yet.
//!
//! Exit status: 0 when the command succeeded, 1 when it found errors, 2 on wrong usage or a
//! command that is not built yet.

use std::path::Path;
use std::process::ExitCode;

use cosmosis::diagnostic::{Diagnostic, nearest};
use cosmosis::layout;
use cosmosis::manifest::{MANIFEST_FILE, Manifest};
use cosmosis::{schema_check, source};

/// Every command in the spec: name, what it does, and whether it is built.
const COMMANDS: [(&str, &str, bool); 15] = [
    ("new", "Start a project from a template", true),
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

Checked: the manifest (cosmosis.noll: its name, null-format and other fields); the layout (the
root's Null files are files, a project has project.noll, every .noll file is UTF-8 text, media/
and data/ files are named by their SHA-256); every Null file's syntax and names, with Null's
parser; and what the files mean, with Null's checker against Blender's schema.

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
        "new" => new(&args[1..]),
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

const NEW_HELP: &str = "Usage: sau new empty <path>

Starts a project in <path>, a folder that does not exist yet or is empty: its manifest
(cosmosis.noll, the package named after the folder), project.noll, and the .gitignore and
.gitattributes Cosmósis ships (Sau's cache but its ID map, view.noll and renders/ kept out of
Git; media/ and data/ in Git LFS), and .blender_project/project.toml, which makes it a Blender
project too, named as the package (Sau keeps it in step with cosmosis.noll). The folder's name, without .sau, gives the package's name
in kebab case: \"Harbor at Dusk.sau\" is package \"harbor-at-dusk\".

Templates: `empty`, the only one so far; the others of Cosmósis spec section 6 come later.

Exit status: 0 when the project was made, 1 when it could not be written, 2 on wrong usage.
";

/// `sau new empty <path>` (Sau task 1d-04e): a project with the shipped Git files.
fn new(args: &[String]) -> ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{NEW_HELP}");
        return ExitCode::SUCCESS;
    }
    let [template, path] = args else {
        return usage_error("`sau new` takes a template and a path: `sau new empty <path>`");
    };
    if template != "empty" {
        return usage_error(&format!(
            "no template `{template}` yet; `empty` is the only one (Cosmósis spec, section 6)"
        ));
    }
    let root = Path::new(path);
    if root.exists()
        && (!root.is_dir() || std::fs::read_dir(root).is_ok_and(|mut d| d.next().is_some()))
    {
        return usage_error(&format!("{path} exists and is not an empty folder"));
    }
    let stem = root
        .file_name()
        .map(|n| {
            let n = n.to_string_lossy();
            n.strip_suffix(".sau").unwrap_or(&n).to_string()
        })
        .unwrap_or_default();
    let Some(name) = cosmosis::manifest::to_kebab(&stem) else {
        return usage_error(&format!(
            "the folder's name {stem:?} gives no kebab-case package name; name the folder in words"
        ));
    };
    let files = [
        (
            cosmosis::manifest::MANIFEST_FILE,
            format!(
                "package {} {{\n  null-format 1\n}}\n",
                null_syntax::escape(&name)
            ),
        ),
        (
            cosmosis::layout::PROJECT_FILE,
            // The folder's name as written, escaped: it may hold `"` or `\`.
            format!("project {} {{\n}}\n", null_syntax::escape(&stem)),
        ),
        (".gitignore", cosmosis::templates::GITIGNORE.to_string()),
        (
            ".gitattributes",
            cosmosis::templates::GITATTRIBUTES.to_string(),
        ),
    ];
    // And Blender's project file, named as the package (S45; Sau task 1d-04c).
    let written = std::fs::create_dir_all(root)
        .and_then(|()| {
            files
                .iter()
                .try_for_each(|(file, text)| std::fs::write(root.join(file), text))
        })
        .and_then(|()| cosmosis::blender_project::write(root, &name).map(|_| ()));
    if let Err(e) = written {
        eprintln!("sau new: cannot write {path}: {e}");
        return ExitCode::FAILURE;
    }
    println!("sau new: made package \"{name}\" in {path}");
    ExitCode::SUCCESS
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
            report(&[*d]);
            return ExitCode::FAILURE;
        }
    };
    // The manifest first: its kind decides what the layout needs (a project has project.noll).
    let manifest_path = root.join(MANIFEST_FILE);
    let mut diagnostics = Vec::new();
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => match Manifest::parse(&text) {
            Ok(m) => Some(m),
            Err(found) => {
                diagnostics.extend(found);
                None
            }
        },
        Err(_) => None, // Not a file, unreadable or not UTF-8: the layout check says which.
    };
    let found = layout::check(&root, manifest.as_ref().map(|m| m.kind));
    diagnostics.extend(found.diagnostics.iter().cloned());
    // Every other Null file through Null's parser: syntax and kebab-case names.
    let mut broken = std::collections::HashSet::new();
    for file in found
        .sources
        .iter()
        .filter(|f| f.path != Path::new(MANIFEST_FILE))
    {
        let parsed = source::parse(&file.text, &file.path);
        if !parsed.diagnostics.is_empty() {
            broken.insert(file.path.clone());
        }
        diagnostics.extend(parsed.diagnostics);
    }
    // Then what they mean, with Null's checker against Blender's schema (task 1a-15). Every
    // file is checked, so handles resolve across files, but a file the parser already found
    // wrong reports only that, as the manifest does: one mistake, one error.
    diagnostics.extend(
        schema_check::check(&found.sources)
            .into_iter()
            .filter(|d| d.file.as_ref().is_none_or(|f| !broken.contains(f))),
    );
    report(&diagnostics);
    let errors = diagnostics.iter().filter(|d| d.is_error()).count();
    if errors > 0 {
        let s = if errors == 1 { "" } else { "s" };
        eprintln!("sau check: {errors} error{s} in {}", root.display());
        return ExitCode::FAILURE;
    }
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    let (n, c) = (found.sources.len(), found.content_files);
    let m = manifest.expect("no errors means the manifest was read");
    println!(
        "sau check: {}: package \"{}\" ({}, Null format {}): {n} Null file{} check, {c} content file{}",
        root.display(),
        m.name,
        m.kind.as_str(),
        m.null_format,
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
