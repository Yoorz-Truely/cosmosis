//! `sau rename` (Sau task 1b-01h, S17) over the corpus: in a copy of each project, every handle a
//! file defines is renamed in turn. Every changed byte is that handle turned into the new one (or
//! the new handle written before a display name), the project still passes `sau check`, and the
//! handle map keeps the datablock's ID under the new handle.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

use null_syntax::ast::{Entry, Value};
use null_syntax::names::to_kebab_case;

fn sau(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sau"))
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Every project of the corpus, slice and project alike.
fn projects() -> Vec<PathBuf> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let mut v = Vec::new();
    for set in ["slice", "project"] {
        for e in fs::read_dir(base.join(set)).unwrap() {
            let p = e.unwrap().path();
            if p.join("cosmosis.noll").is_file() {
                v.push(p);
            }
        }
    }
    v.sort();
    assert!(v.len() >= 6, "{v:?}");
    v
}

/// Every file under `dir`, relative to `root`, with its text.
fn files(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            files(root, &p, out);
        } else {
            let rel = p.strip_prefix(root).unwrap().to_path_buf();
            out.push((rel, fs::read_to_string(&p).unwrap()));
        }
    }
    out.sort();
}

fn copy_dir(from: &Path, to: &Path) {
    copy(from, to);
}

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let p = e.unwrap().path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy(&p, &dest);
        } else {
            fs::copy(&p, &dest).unwrap();
        }
    }
}

/// The (kind, handle) of every top-level datablock the project's `.noll` files define, written or
/// made from the display name, with whether it was written.
fn definitions(project: &Path) -> Vec<(String, String, bool)> {
    let mut all = Vec::new();
    files(project, project, &mut all);
    let mut out = Vec::new();
    for (path, source) in &all {
        if path.extension().is_none_or(|x| x != "noll")
            || ["cosmosis.noll", "settings.noll", "view.noll"].contains(&path.to_str().unwrap())
            || path.starts_with(".sau")
        {
            continue;
        }
        let parse = null_syntax::parse(source);
        for node in parse.document().nodes().filter(|n| n.name() != "project") {
            let first = node.entries().find_map(|e| match e {
                Entry::Arg(v) => Some(v),
                _ => None,
            });
            let (handle, written) = match first {
                Some(Value::Word(t)) if t.text.starts_with('@') => (t.text.to_string(), true),
                Some(v @ (Value::Str(_) | Value::Raw(_))) => {
                    let name = v.as_string().unwrap();
                    (format!("@{}", to_kebab_case(&name).unwrap()), false)
                }
                _ => continue,
            };
            out.push((node.name().into_owned(), handle, written));
        }
    }
    out
}

/// A handle map with a made-up ID for each definition.
fn map(defs: &[(String, String, bool)]) -> String {
    let mut lines: Vec<String> = defs
        .iter()
        .enumerate()
        .map(|(i, (kind, handle, _))| format!("  {kind} {handle} \"{:032x}\"", i + 1))
        .collect();
    lines.sort();
    lines.dedup();
    format!("ids {{\n{}\n}}\n", lines.join("\n"))
}

#[test]
fn every_handle_of_every_corpus_project_renames_cleanly() {
    static N: AtomicUsize = AtomicUsize::new(0);
    let (mut renamed, mut refused) = (0, 0);
    for project in projects() {
        let defs = definitions(&project);
        assert!(!defs.is_empty(), "{}", project.display());
        for (kind, old, written) in &defs {
            // The kinds sharing this handle: a rename of a shared handle is refused, by design.
            let kinds = defs.iter().filter(|(_, h, _)| h == old).count();
            let new = format!("{old}-renamed");
            let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
                "rename-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&dir);
            copy(&project, &dir);
            fs::create_dir_all(dir.join(".sau")).unwrap();
            fs::write(dir.join(".sau/ids.noll"), map(&defs)).unwrap();
            let mut before = Vec::new();
            files(&dir, &dir, &mut before);

            let out = sau(&["rename", old, &new, dir.to_str().unwrap()]);
            let what = format!("{} {kind} {old}", project.display());
            if kinds > 1 {
                assert_eq!(
                    out.status.code(),
                    Some(1),
                    "{what}: a shared handle is refused"
                );
                assert!(text(&out.stderr).contains("share the handle"), "{what}");
                let mut after = Vec::new();
                files(&dir, &dir, &mut after);
                assert_eq!(before, after, "{what}: a refused rename changes nothing");
                refused += 1;
                continue;
            }
            assert!(
                out.status.success(),
                "{what}: {}{}",
                text(&out.stdout),
                text(&out.stderr)
            );
            renamed += 1;

            // Every change is the handle made the new one, or the new one before a name.
            let mut after = Vec::new();
            files(&dir, &dir, &mut after);
            assert_eq!(before.len(), after.len(), "{what}");
            let mut occurrences = 0;
            for ((path, was), (_, now)) in before.iter().zip(&after) {
                if path.ends_with("ids.noll") {
                    continue;
                }
                assert!(!was.contains(&new), "{what}: {new} was there already");
                occurrences += now.matches(new.as_str()).count();
                // A handle made from the name was written before it: take that out first.
                let unwritten = if *written {
                    now.clone()
                } else {
                    now.replace(&format!("{new} \""), "\"")
                };
                let back = unwritten.replace(new.as_str(), old);
                assert_eq!(&back, was, "{what}: {} changed otherwise", path.display());
            }
            // The definition and each reference the summary counts.
            let summary = text(&out.stdout);
            let refs: usize = summary
                .split("its definition and ")
                .nth(1)
                .and_then(|s| s.split(' ').next())
                .and_then(|n| n.parse().ok())
                .unwrap_or_else(|| panic!("{what}: {summary}"));
            assert_eq!(occurrences, refs + 1, "{what}: {summary}");
            if !written {
                assert!(
                    summary.contains(&format!("{kind} {old} is {new}")),
                    "{what}"
                );
            }

            // Still a valid project, with every reference resolving.
            let check = sau(&["check", dir.to_str().unwrap()]);
            assert!(
                check.status.success(),
                "{what}: sau check after the rename: {}{}",
                text(&check.stdout),
                text(&check.stderr)
            );

            // The map: the same ID, now under the new handle.
            let ids = fs::read_to_string(dir.join(".sau/ids.noll")).unwrap();
            let (want, _) = cosmosis::ids::parse(&map(&defs));
            let (got, bad) = cosmosis::ids::parse(&ids);
            assert!(bad.is_empty(), "{what}: {ids}");
            let id = &want[&(kind.clone(), old.clone())];
            assert_eq!(
                got.get(&(kind.clone(), new.clone())),
                Some(id),
                "{what}: {ids}"
            );
            assert!(!got.contains_key(&(kind.clone(), old.clone())), "{what}");
            assert_eq!(got.len(), want.len(), "{what}");
            let _ = fs::remove_dir_all(&dir);
        }
    }
    eprintln!("sau rename: {renamed} handles renamed, {refused} shared ones refused");
    assert!(renamed >= 20, "only {renamed} renames");
}

#[test]
fn a_rename_is_refused_when_the_map_has_a_line_it_cannot_read() {
    let project = projects().into_iter().next().unwrap();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("rename-bad-map-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy(&project, &dir);
    let defs = definitions(&dir);
    let (_, old, _) = &defs[0];
    fs::create_dir_all(dir.join(".sau")).unwrap();
    fs::write(
        dir.join(".sau/ids.noll"),
        "ids {\n  object @x \"not hex\"\n}\n",
    )
    .unwrap();
    let mut before = Vec::new();
    files(&dir, &dir, &mut before);
    let out = sau(&["rename", old, "@fresh-handle", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("cannot read"),
        "{}",
        text(&out.stderr)
    );
    let mut after = Vec::new();
    files(&dir, &dir, &mut after);
    assert_eq!(before, after);
    let _ = fs::remove_dir_all(&dir);
}

/// A copy of the first corpus project, its first definition's handle, and every file's text.
fn fresh(tag: &str) -> (PathBuf, String) {
    let project = projects().into_iter().next().unwrap();
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("rename-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy(&project, &dir);
    let old = definitions(&dir)[0].1.clone();
    (dir, old)
}

#[test]
fn a_map_that_exists_but_cannot_be_read_refuses_the_rename_and_changes_nothing() {
    // Only a missing map means no map (1b-01h's review): a folder where the map should be is
    // not "no map", so the rename is refused before any file is written.
    let (dir, old) = fresh("map-folder");
    fs::create_dir_all(dir.join(".sau/ids.noll")).unwrap();
    let mut before = Vec::new();
    files(&dir, &dir, &mut before);
    let out = sau(&["rename", &old, "@fresh-handle", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stdout));
    assert!(
        text(&out.stderr).contains("nothing") || text(&out.stderr).contains("fix it first"),
        "{}",
        text(&out.stderr)
    );
    let mut after = Vec::new();
    files(&dir, &dir, &mut after);
    assert_eq!(before, after);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_sau_folder_that_is_a_link_refuses_the_rename_before_any_file_is_written() {
    let (dir, old) = fresh("map-link");
    let elsewhere = dir.with_extension("elsewhere");
    let _ = fs::remove_dir_all(&elsewhere);
    fs::create_dir_all(&elsewhere).unwrap();
    let defs = definitions(&dir);
    fs::write(elsewhere.join("ids.noll"), map(&defs)).unwrap();
    std::os::unix::fs::symlink(&elsewhere, dir.join(".sau")).unwrap();
    let mut before = Vec::new();
    files(&dir, &dir, &mut before);
    let out = sau(&["rename", &old, "@fresh-handle", dir.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("is a link"),
        "{}",
        text(&out.stderr)
    );
    let mut after = Vec::new();
    files(&dir, &dir, &mut after);
    assert_eq!(before, after);
    assert_eq!(
        fs::read_to_string(elsewhere.join("ids.noll")).unwrap(),
        map(&defs)
    );
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&elsewhere);
}

#[test]
fn no_staged_file_is_left_behind() {
    let (dir, old) = fresh("staged");
    let defs = definitions(&dir);
    fs::create_dir_all(dir.join(".sau")).unwrap();
    fs::write(dir.join(".sau/ids.noll"), map(&defs)).unwrap();
    let out = sau(&["rename", &old, "@fresh-handle", dir.to_str().unwrap()]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let mut after = Vec::new();
    files(&dir, &dir, &mut after);
    let left: Vec<_> = after
        .iter()
        .filter(|(p, _)| p.to_string_lossy().contains(".sau-"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_block_of_sub_objects_of_a_map_entry_survives_a_rename() {
    // S73 (4): the owner's block of sub-object entries moves with its entry.
    let (dir, old) = fresh("map-block");
    let defs = definitions(&dir);
    let kind = defs[0].0.clone();
    let block = "{\n    modifier @bevel 7\n    vertex-group @left \"Left\"\n  }";
    let with_block = map(&defs).replacen(
        &format!("  {kind} {old} \"{:032x}\"\n", 1),
        &format!("  {kind} {old} \"{:032x}\" {block}\n", 1),
        1,
    );
    assert_ne!(
        with_block,
        map(&defs),
        "the first entry is the first definition's"
    );
    fs::create_dir_all(dir.join(".sau")).unwrap();
    fs::write(dir.join(".sau/ids.noll"), &with_block).unwrap();
    let out = sau(&["rename", &old, "@fresh-handle", dir.to_str().unwrap()]);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let ids = fs::read_to_string(dir.join(".sau/ids.noll")).unwrap();
    assert!(
        ids.contains(&format!("  {kind} @fresh-handle \"{:032x}\" {block}\n", 1)),
        "{ids}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn every_state_a_running_sau_can_load_between_the_renames_is_safe() {
    // 1b-01h round 3: `sau rename` replaces the map first, then the files in their paths'
    // order, and a running Sau may load between any two. With the map at the new handle and
    // the files at the old one, apply keeps the datablock (the old handle still defines it);
    // with the files partly renamed, the project fails the check, so nothing is applied. Only
    // the first and the last state check clean, and no state has every file renamed with the
    // map still at the old handle.
    let (dir, _) = fresh("states");
    let defs = definitions(&dir);
    fs::create_dir_all(dir.join(".sau")).unwrap();
    fs::write(dir.join(".sau/ids.noll"), map(&defs)).unwrap();
    let mut before = Vec::new();
    files(&dir, &dir, &mut before);
    // A handle named in more than one file, so there are states in between.
    let (old, renamed_dir, after) = defs
        .iter()
        .find_map(|(_, old, _)| {
            let copy = dir.with_extension(format!("try{}", &old[1..]));
            let _ = fs::remove_dir_all(&copy);
            copy_dir(&dir, &copy);
            let out = sau(&["rename", old, "@renamed-here", copy.to_str().unwrap()]);
            assert!(out.status.success(), "{}", text(&out.stderr));
            let mut after = Vec::new();
            files(&copy, &copy, &mut after);
            let changed = before
                .iter()
                .zip(&after)
                .filter(|(b, a)| b != a && !b.0.ends_with("ids.noll"))
                .count();
            (changed > 1).then(|| (old.clone(), copy, after))
        })
        .expect("a handle named in two files");
    let map_after = after.iter().find(|(p, _)| p.ends_with("ids.noll")).unwrap();
    let file_order: Vec<usize> = (0..before.len())
        .filter(|&i| before[i] != after[i] && !before[i].0.ends_with("ids.noll"))
        .collect();
    // The states in the order `sau rename` makes them: the map, then each file by path.
    let state_dir = dir.with_extension("state");
    for k in 0..=file_order.len() {
        let _ = fs::remove_dir_all(&state_dir);
        copy_dir(&dir, &state_dir);
        fs::write(state_dir.join(&map_after.0), &map_after.1).unwrap();
        for &i in &file_order[..k] {
            fs::write(state_dir.join(&after[i].0), &after[i].1).unwrap();
        }
        let check = sau(&["check", state_dir.to_str().unwrap()]);
        let first_or_last = k == 0 || k == file_order.len();
        if !first_or_last {
            assert!(
                !check.status.success(),
                "{old}: {k} of {} files renamed checks clean, so a running Sau would apply it",
                file_order.len()
            );
        }
        if k == file_order.len() {
            assert!(check.status.success(), "{}", text(&check.stderr));
        }
    }
    for d in [&dir, &renamed_dir, &state_dir] {
        let _ = fs::remove_dir_all(d);
    }
}

#[test]
fn rename_takes_two_handles() {
    let out = sau(&["rename", "@a"]);
    assert_eq!(out.status.code(), Some(2));
    let out = sau(&["rename", "--help"]);
    assert!(out.status.success());
    assert!(text(&out.stdout).contains("sau rename"));
}
