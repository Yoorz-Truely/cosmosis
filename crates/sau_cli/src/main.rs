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

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cosmosis::diagnostic::{Diagnostic, nearest};
use cosmosis::layout;
use cosmosis::manifest::{MANIFEST_FILE, Manifest};
use cosmosis::{schema_check, source};

/// Every command in the spec: name, what it does, and whether it is built.
const COMMANDS: [(&str, &str, bool); 16] = [
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
    ("fmt", "Print files in canonical form", true),
    (
        "rename",
        "Rename a handle in every file that names it",
        true,
    ),
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
        "fmt" => fmt(&args[1..]),
        "rename" => rename(&args[1..]),
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
Git; media/ and data/ in Git LFS). The folder's name, without .sau, gives the package's name
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
    let written = std::fs::create_dir_all(root).and_then(|()| {
        files
            .iter()
            .try_for_each(|(file, text)| std::fs::write(root.join(file), text))
    });
    if let Err(e) = written {
        eprintln!("sau new: cannot write {path}: {e}");
        return ExitCode::FAILURE;
    }
    println!("sau new: made package \"{name}\" in {path}");
    ExitCode::SUCCESS
}

const FMT_HELP: &str = "Usage: sau fmt [--check] [path]

Writes every Null file of the project holding <path> (default: the current folder) in canonical
form, with Null's canonical printer: each block's lines and each line's key=values in Blender's
schema order, Null's forms where the property they write stands, comments kept with their lines,
then the formatter's layout (Null spec, section 15). The manifest's fields go in the order of
Cosmósis spec section 4; settings.noll, view.noll and .sau/ids.noll get the layout only. A file that does not parse keeps its order. Formatting twice
changes nothing.

  --check   write nothing: list the files that are not in canonical form

Exit status: 0 when every file is canonical (or was made so), 1 when --check found a file that is
not, or a file could not be written, 2 on wrong usage.
";

/// `sau fmt [--check] [path]` (Sau task C-07): the project's files in canonical form.
fn fmt(args: &[String]) -> ExitCode {
    let mut path = None;
    let mut check_only = false;
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{FMT_HELP}");
                return ExitCode::SUCCESS;
            }
            "--check" => check_only = true,
            flag if flag.starts_with('-') => {
                return usage_error(&format!("`sau fmt` has no option `{flag}`"));
            }
            p if path.is_none() => path = Some(p),
            p => return usage_error(&format!("`sau fmt` takes one path; `{p}` is a second")),
        }
    }
    let root = match layout::find_root(Path::new(path.unwrap_or("."))) {
        Ok(root) => root,
        Err(d) => {
            report(&[*d]);
            return ExitCode::FAILURE;
        }
    };
    let found = layout::check(&root, None);
    let schema = schema_check::schema();
    let mut changed = Vec::new();
    for file in &found.sources {
        // Datablock files in the schema's order; the others, whose forms are their own, in the
        // formatter's layout alone (S66).
        let text = if layout::holds_datablocks(&file.path) {
            null_print::canonical(&file.text, schema)
        } else if file.path == Path::new(MANIFEST_FILE) {
            // The manifest's fields in the spec's order (section 4), then the layout.
            null_print::format(&cosmosis::manifest::canonical_order(&file.text))
        } else {
            null_print::format(&file.text)
        };
        if text == file.text {
            continue;
        }
        if !check_only {
            // Written through a link to its target, as write-back does (1a-14).
            if let Err(e) = std::fs::write(root.join(&file.path), &text) {
                eprintln!("sau fmt: cannot write {}: {e}", file.path.display());
                return ExitCode::FAILURE;
            }
        }
        changed.push(file.path.display().to_string());
    }
    let n = found.sources.len();
    if check_only {
        for f in &changed {
            println!("{f}");
        }
        if !changed.is_empty() {
            eprintln!(
                "sau fmt --check: {} of {n} Null files in {} are not in canonical form; `sau fmt` writes them",
                changed.len(),
                root.display()
            );
            return ExitCode::FAILURE;
        }
        println!(
            "sau fmt --check: {n} Null files in {} are canonical",
            root.display()
        );
        return ExitCode::SUCCESS;
    }
    for f in &changed {
        println!("formatted {f}");
    }
    println!(
        "sau fmt: {} of {n} Null files in {} formatted",
        changed.len(),
        root.display()
    );
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

const RENAME_HELP: &str = "Usage: sau rename @old @new [path]

Renames the datablock handle @old to @new in every file of the project holding <path> (default:
the current folder) that names it, as an editor renames a symbol (Sau's decision S17): its
definition, every reference, every path that starts at it and every pattern's copy of it, and
nothing that only looks the same (a sub-object or node with that handle, a string, a comment).
The handle map, .sau/ids.noll, keeps the datablock's ID under the new handle, so its identity in
Sau and in every .blend stays.

Refused, with nothing written, when @new is not a handle, no datablock is @old, datablocks of two
kinds share @old, one of the same kind already is @new, or a file has syntax errors.

Exit status: 0 when renamed, 1 when refused, 2 on wrong usage.
";

/// `sau rename @old @new [path]` (task 1b-01h).
fn rename(args: &[String]) -> ExitCode {
    let mut words = Vec::new();
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{RENAME_HELP}");
                return ExitCode::SUCCESS;
            }
            flag if flag.starts_with('-') => {
                return usage_error(&format!("`sau rename` has no option `{flag}`"));
            }
            w => words.push(w),
        }
    }
    let (old, new, path) = match words.as_slice() {
        [old, new] => (*old, *new, "."),
        [old, new, path] => (*old, *new, *path),
        _ => return usage_error("`sau rename` takes @old, @new and an optional path"),
    };
    let root = match layout::find_root(Path::new(path)) {
        Ok(root) => root,
        Err(d) => {
            report(&[*d]);
            return ExitCode::FAILURE;
        }
    };
    let found = layout::check(&root, None);
    let files: Vec<&layout::SourceFile> = found
        .sources
        .iter()
        .filter(|f| layout::holds_datablocks(&f.path))
        .collect();
    let parses: Vec<null_syntax::Parse> =
        files.iter().map(|f| null_syntax::parse(&f.text)).collect();
    let list: Vec<(null_syntax::FileId, &null_syntax::Parse)> = parses
        .iter()
        .enumerate()
        .map(|(i, p)| (null_syntax::FileId(i as u32), p))
        .collect();
    let done = match null_check::rename::rename(&list, old, new) {
        Ok(done) => done,
        Err(null_check::rename::RenameError::Unparsed(id)) => {
            eprintln!(
                "sau rename: {} has syntax errors; fix them first (`sau check`), so that every reference can be found",
                files[id.0 as usize].path.display()
            );
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("sau rename: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Everything is checked before anything is written: the map first (only a missing map is
    // no map; a link, an unreadable file or a line it cannot use refuses the rename, since
    // writing it again would lose what it holds), then every file's new text is staged beside
    // its target, and only then is each renamed over its file. The map goes first: a running Sau
    // may load between two of these renames, and with the map at the new handle and the files at
    // the old one nothing changes, while files only partly renamed fail the check, so nothing is
    // applied. The map last would let a load see every file renamed and the map not yet, make a
    // new datablock for the new handle and remove the old one as stale (1b-01h round 3).
    let mut map = match cosmosis::ids::read(&root) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("sau rename: {e}; fix it first (`sau check`), so that no ID is lost");
            return ExitCode::FAILURE;
        }
    };
    let map_text = map.as_mut().and_then(|m| {
        cosmosis::ids::rename(m, &done.kind, old, new).then(|| cosmosis::ids::text(m))
    });
    let mut staged: Vec<(PathBuf, PathBuf, String)> = Vec::new();
    let unstage = |staged: &[(PathBuf, PathBuf, String)]| {
        for (temporary, _, _) in staged {
            let _ = std::fs::remove_file(temporary);
        }
    };
    for (i, file) in files.iter().enumerate() {
        // Each file's edits, from its end, so earlier spans stay where they were.
        let mut edits: Vec<_> = done
            .edits
            .iter()
            .filter(|e| e.file.0 as usize == i)
            .collect();
        if edits.is_empty() {
            continue;
        }
        edits.sort_by_key(|e| std::cmp::Reverse(e.span.start));
        let mut text = file.text.clone();
        for e in edits {
            text.replace_range(e.span.start..e.span.end, &e.replacement);
        }
        // Written through a link to its target, as write-back does (1a-14): staged beside the
        // target, then renamed over it.
        let staged_one = std::fs::canonicalize(root.join(&file.path)).and_then(|target| {
            let mut name = target.file_name().unwrap_or_default().to_os_string();
            name.push(".sau-rename");
            let temporary = target.with_file_name(name);
            std::fs::write(&temporary, &text)?;
            std::fs::File::open(&temporary)?.sync_all()?;
            Ok((temporary, target))
        });
        match staged_one {
            Ok((temporary, target)) => {
                staged.push((temporary, target, file.path.display().to_string()));
            }
            Err(e) => {
                unstage(&staged);
                eprintln!(
                    "sau rename: cannot write {}: {e}; nothing was changed",
                    file.path.display()
                );
                return ExitCode::FAILURE;
            }
        }
    }
    let staged_map = match map_text.as_deref().map(|t| cosmosis::ids::stage(&root, t)) {
        Some(Ok(path)) => Some(path),
        Some(Err(e)) => {
            unstage(&staged);
            eprintln!(
                "sau rename: cannot write {}: {e}; nothing was changed",
                layout::IDS_FILE
            );
            return ExitCode::FAILURE;
        }
        None => None,
    };
    // The map first, then the files in their paths' order.
    let mut map_note = String::new();
    if let Some(m) = &staged_map {
        if let Err(e) = cosmosis::ids::commit(&root, m) {
            unstage(&staged);
            let _ = std::fs::remove_file(m);
            eprintln!(
                "sau rename: cannot replace {}: {e}; nothing was changed",
                layout::IDS_FILE
            );
            return ExitCode::FAILURE;
        }
        map_note = format!(", and its ID in {}", layout::IDS_FILE);
    }
    staged.sort_by(|a, b| a.2.cmp(&b.2));
    let mut changed = Vec::new();
    for (n, (temporary, target, shown)) in staged.iter().enumerate() {
        if let Err(e) = std::fs::rename(temporary, target) {
            unstage(&staged[n..]);
            eprintln!(
                "sau rename: cannot replace {shown}: {e}; the map and {} file{} already renamed: {}",
                changed.len(),
                if changed.len() == 1 { " was" } else { "s were" },
                changed.join(", ")
            );
            return ExitCode::FAILURE;
        }
        changed.push(shown.clone());
    }
    println!(
        "sau rename: {} {old} is {new}: its definition and {} reference{} in {} file{}{map_note}",
        done.kind,
        done.references,
        if done.references == 1 { "" } else { "s" },
        changed.len(),
        if changed.len() == 1 { "" } else { "s" },
    );
    for f in changed {
        println!("  {f}");
    }
    ExitCode::SUCCESS
}
