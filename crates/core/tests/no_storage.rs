//! The brief: "nothing is stored between visits: the page uses no browser
//! storage". This reads the page's own files and fails if any names a browser
//! storage API. Adding storage is a decision for the person, not the agent.

use std::fs;
use std::path::Path;

const STORAGE_NAMES: &[&str] = &[
    "localStorage",
    "sessionStorage",
    "indexedDB",
    "IDBFactory",
    "document.cookie",
    "caches",
    "CacheStorage",
    "serviceWorker",
    "navigator.storage",
    "openDatabase",
];

fn storage_names_in(text: &str) -> Vec<&'static str> {
    STORAGE_NAMES
        .iter()
        .copied()
        .filter(|name| text.contains(name))
        .collect()
}

#[test]
fn the_page_names_no_browser_storage() {
    let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let mut found = Vec::new();
    for entry in fs::read_dir(&web).expect("read web/") {
        let path = entry.expect("entry").path();
        if path.is_file() {
            let text = fs::read_to_string(&path).expect("read page file");
            for name in storage_names_in(&text) {
                found.push(format!("{}: {name}", path.display()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "browser storage in the page:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scanner_sees_storage() {
    assert_eq!(
        storage_names_in("window.localStorage.setItem('n', name)"),
        vec!["localStorage"]
    );
    assert_eq!(storage_names_in("indexedDB.open('x')"), vec!["indexedDB"]);
    assert!(storage_names_in("el(\"note\").value").is_empty());
}
