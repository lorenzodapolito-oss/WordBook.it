//! Thesauri in the MyThes format, read at runtime from files installed on the system.
//!
//! Like the Hunspell spelling dictionaries ([`crate::hunspell`]), a thesaurus is **not bundled**:
//! the Italian one (`th_it_IT_v2.dat`, from LibreOffice's `dict-it` extension or the `mythes-it`
//! package) is licensed under the LGPL, so WordCraft reads the user's own copy and ships none of
//! its data. Without a file the language simply has no synonyms.
//!
//! Format (MyThes `.dat`): the first line names the encoding (`UTF-8`, `ISO8859-1`); then, for each
//! word, a head line `word|N` followed by `N` meaning lines `(label)|synonym|synonym|…`. The
//! `.idx` file that goes with it only lists where each head line starts, so it is not needed:
//! the index is built from the `.dat` in one pass (a few milliseconds for the Italian one).
//!
//! Search order: the same folders as the dictionaries ([`crate::hunspell::search_dirs`]: the
//! dictionaries folder setting, `WORDCRAFT_DICTIONARIES`, the system and LibreOffice folders),
//! then the usual `mythes` folders.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::lang::ProofLang;

/// Largest `.dat` file read (the Italian one is about 2 MB).
const MAX_FILE: u64 = 64 << 20;
/// Most meanings and synonyms returned for one word.
const MAX_MEANINGS: usize = 24;
const MAX_SYNONYMS: usize = 60;

/// One sense of a word: its label (`s.m.`, `v.`, `agg. Avvilito`…) and the synonyms of that sense.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meaning {
    pub label: String,
    pub synonyms: Vec<String>,
}

/// A thesaurus: the decoded `.dat` text and, per lowercase word, where its entries start.
#[derive(Default)]
pub struct Thesaurus {
    text: String,
    index: HashMap<String, Vec<usize>>,
}

impl Thesaurus {
    /// Read a MyThes `.dat` (any of the encodings it declares on its first line).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let first = bytes.split(|&b| b == b'\n').next().unwrap_or_default();
        let enc = String::from_utf8_lossy(first).trim().to_ascii_uppercase().replace(['-', '_'], "");
        let text = if matches!(enc.as_str(), "ISO88591" | "ISO885915" | "LATIN1" | "WINDOWS1252" | "CP1252") {
            bytes.iter().map(|&b| char::from(b)).collect()
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        };
        Self::from_text(text)
    }

    /// Build from decoded text (the encoding line included).
    pub fn from_text(text: String) -> Self {
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        let mut off = 0usize;
        for (i, line) in text.split('\n').enumerate() {
            let at = off;
            off += line.len() + 1;
            if i == 0 {
                continue; // the encoding line
            }
            if let Some((word, _)) = head(line) {
                index.entry(normalize(word)).or_default().push(at);
            }
        }
        Thesaurus { text, index }
    }

    /// No entries (no file found, or an unreadable one).
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Number of distinct words.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// The senses of `word`, or of a base form of it when the word as typed has none (`belle` →
    /// `bello`, `amici` → `amico`, `l'amico` → `amico`). Empty when the thesaurus doesn't know it.
    pub fn lookup(&self, word: &str) -> Vec<Meaning> {
        for cand in candidates(word) {
            let Some(offs) = self.index.get(&cand) else { continue };
            let mut out: Vec<Meaning> = Vec::new();
            for &off in offs {
                self.read_entry(off, &cand, &mut out);
            }
            if !out.is_empty() {
                return out;
            }
        }
        Vec::new()
    }

    fn read_entry(&self, off: usize, word: &str, out: &mut Vec<Meaning>) {
        let Some(rest) = self.text.get(off..) else { return };
        let mut lines = rest.split('\n');
        let Some((_, n)) = lines.next().and_then(head) else { return };
        for line in lines.take(n.min(MAX_MEANINGS)) {
            let mut fields = line.trim_end_matches('\r').split('|');
            let label = fields.next().unwrap_or("").trim().trim_start_matches('(').trim_end_matches(')').trim().to_string();
            let mut synonyms: Vec<String> = Vec::new();
            for f in fields {
                // Some thesauri add a note in brackets after a synonym: "(generic term)".
                let syn = f.split(" (").next().unwrap_or(f).trim();
                if !syn.is_empty() && normalize(syn) != word && !synonyms.iter().any(|s| s == syn) {
                    synonyms.push(syn.to_string());
                }
                if synonyms.len() >= MAX_SYNONYMS {
                    break;
                }
            }
            if !synonyms.is_empty() && out.len() < MAX_MEANINGS {
                out.push(Meaning { label, synonyms });
            }
        }
    }
}

/// A head line `word|N` (not a meaning line, which starts with `(`).
fn head(line: &str) -> Option<(&str, usize)> {
    let line = line.trim_end_matches('\r');
    if line.starts_with('(') {
        return None;
    }
    let (word, n) = line.rsplit_once('|')?;
    let n = n.trim().parse::<usize>().ok()?;
    (!word.trim().is_empty()).then_some((word.trim(), n))
}

fn normalize(word: &str) -> String {
    word.trim().replace('\u{2019}', "'").to_lowercase()
}

/// Forms to look up, most likely first: the word, the word after an elision (`l'amico`), and
/// the same stem with another Italian ending (`belle` → `bello`; `amici` → `amico`).
fn candidates(word: &str) -> Vec<String> {
    let w = normalize(word);
    let base = match w.rsplit_once('\'') {
        Some((_, after)) if !after.is_empty() => after,
        _ => w.trim_end_matches('\''),
    };
    // The word as typed first (`po'` keeps its apostrophe), then without the elided article.
    let mut out = vec![w.clone()];
    let mut push = |s: String| {
        if !out.contains(&s) {
            out.push(s);
        }
    };
    push(base.to_string());
    if let Some(stem) = base.strip_suffix("chi") {
        push(format!("{stem}co"));
    }
    if let Some(stem) = base.strip_suffix("ghi") {
        push(format!("{stem}go"));
    }
    // Feminine plurals in -che / -ghe: amiche → amica → amico.
    for (end, singular) in [("che", "c"), ("ghe", "g")] {
        if let Some(stem) = base.strip_suffix(end) {
            for v in ['a', 'o'] {
                push(format!("{stem}{singular}{v}"));
            }
        }
    }
    if let Some(last) = base.chars().next_back().filter(|c| "aeio".contains(*c)) {
        let stem = base.get(..base.len() - last.len_utf8()).unwrap_or("");
        for v in ['o', 'a', 'e', 'i'] {
            push(format!("{stem}{v}"));
        }
    }
    out
}

/// File names of the Italian thesaurus (LibreOffice's `_v2` first).
const IT_NAMES: &[&str] = &["th_it_IT_v2", "th_it_IT", "th_it"];

/// The Italian thesaurus: `None` until first use, then the installed one (or an empty one).
/// The flag is true for one given with [`install_italian`], which [`reload`] keeps.
static IT: RwLock<Option<(Arc<Thesaurus>, bool)>> = RwLock::new(None);

/// Thesaurus folders besides the dictionary ones (Debian's `mythes-it`, Homebrew).
const EXTRA_DIRS: &[&str] = &["/usr/share/mythes", "/usr/local/share/mythes", "/opt/homebrew/share/mythes", "/Library/Spelling"];

/// The first `<name>.dat` found for any of `names`, searching the dictionary folders (with
/// LibreOffice's extension folder for `lo_name`) and then [`EXTRA_DIRS`].
pub fn find(names: &[&str], lo_name: &str) -> Option<PathBuf> {
    let lo = crate::hunspell::lo_ext_dirs(lo_name);
    let mut dirs = crate::hunspell::search_dirs(Some(&lo));
    if !cfg!(target_arch = "wasm32") {
        dirs.extend(EXTRA_DIRS.iter().map(PathBuf::from));
    }
    dirs.into_iter().find_map(|dir| names.iter().map(|n| dir.join(format!("{n}.dat"))).find(|p| p.is_file()))
}

/// Read a `.dat` from disk (size-capped).
pub fn load(path: &Path) -> Result<Thesaurus, String> {
    let len = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?.len();
    if len > MAX_FILE {
        return Err(format!("{}: too large ({len} bytes)", path.display()));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(Thesaurus::from_bytes(&bytes))
}

/// The `.dat` of the Italian thesaurus found now, if any (File › Options › Proofing).
pub fn italian_path() -> Option<PathBuf> {
    find(IT_NAMES, "it_IT")
}

/// The Italian thesaurus: the installed one (read on first use), or an empty one.
pub fn italian() -> Arc<Thesaurus> {
    if let Some((t, _)) = IT.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return t.clone();
    }
    let loaded = Arc::new(match italian_path() {
        Some(p) => load(&p).unwrap_or_else(|e| {
            log::warn!("Italian thesaurus: {e}");
            Thesaurus::default()
        }),
        None => {
            log::info!("Italian thesaurus: no MyThes th_it_IT installed");
            Thesaurus::default()
        }
    });
    IT.write().unwrap_or_else(|e| e.into_inner()).get_or_insert((loaded, false)).0.clone()
}

/// Use this thesaurus from now on (a `.dat` the user picked, decoded or not).
pub fn install_italian(bytes: &[u8]) {
    *IT.write().unwrap_or_else(|e| e.into_inner()) = Some((Arc::new(Thesaurus::from_bytes(bytes)), true));
}

/// Forget the thesaurus read from disk: the next lookup searches the folders again. One given
/// with [`install_italian`] stays.
pub fn reload() {
    let mut slot = IT.write().unwrap_or_else(|e| e.into_inner());
    if slot.as_ref().is_some_and(|(_, installed)| !installed) {
        *slot = None;
    }
}

/// The thesaurus for a language, when WordCraft reads one for it (Italian).
pub fn for_lang(lang: ProofLang) -> Option<Arc<Thesaurus>> {
    (lang == ProofLang::It).then(italian)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small synthetic thesaurus in MyThes format (our own words, not LibreOffice's data).
    const SAMPLE: &str = "UTF-8\n\
        bello|2\n\
        (agg.)|grazioso|piacevole|splendido\n\
        (agg. Elegante)|elegante|raffinato\n\
        amico|1\n\
        (s.m.)|compagno|sodale (familiare)|bello\n\
        po'|1\n\
        (s.m.)|poco\n\
        casa|1\n\
        (s.f.)|abitazione|dimora\n";

    #[test]
    fn reads_meanings_labels_and_synonyms() {
        let t = Thesaurus::from_text(SAMPLE.to_string());
        assert_eq!(t.len(), 4);
        let m = t.lookup("bello");
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].label, "agg.");
        assert_eq!(m[0].synonyms, ["grazioso", "piacevole", "splendido"]);
        assert_eq!(m[1].label, "agg. Elegante");
        // A note in brackets is dropped, and a word is never its own synonym.
        assert_eq!(t.lookup("amico")[0].synonyms, ["compagno", "sodale", "bello"]);
        assert!(t.lookup("bello").iter().all(|m| !m.synonyms.iter().any(|s| s == "bello")));
    }

    #[test]
    fn finds_inflected_and_elided_forms_and_ignores_case() {
        let t = Thesaurus::from_text(SAMPLE.to_string());
        for w in ["Bello", "BELLO", "bella", "belle", "belli", "l'amico", "l\u{2019}amico", "amici", "amiche", "case"] {
            assert!(!t.lookup(w).is_empty(), "{w}");
        }
        assert!(!t.lookup("po'").is_empty());
        assert!(t.lookup("sconosciuto").is_empty());
        assert!(t.lookup("").is_empty());
    }

    #[test]
    fn reads_latin1_files_and_survives_junk() {
        let mut bytes = b"ISO8859-1\ncitt".to_vec();
        bytes.extend_from_slice(&[0xE0]); // à
        bytes.extend_from_slice(b"|1\n(s.f.)|urbe\n");
        let t = Thesaurus::from_bytes(&bytes);
        assert_eq!(t.lookup("città")[0].synonyms, ["urbe"]);
        // Damaged input never panics: a bad count, a head with no meanings, binary noise.
        for junk in [&b""[..], b"UTF-8\n", b"UTF-8\nx|99999999999999999999\n", b"UTF-8\nx|3\n", b"\xff\xfe\x00|1\n(a)|b", b"UTF-8\n|1\n(a)|b\n"] {
            let t = Thesaurus::from_bytes(junk);
            let _ = t.lookup("x");
            let _ = t.lookup("");
        }
    }

    /// Against the real Italian file, when one is installed: `WORDCRAFT_DICTIONARIES=/path/to/folder
    /// cargo test -p wordcraft-proof -- --ignored real_italian --nocapture` (the file isn't in this repo).
    #[test]
    #[ignore]
    fn real_italian_thesaurus_if_installed() {
        reload();
        let t = italian();
        assert!(!t.is_empty(), "no th_it_IT thesaurus found in the dictionary folders");
        println!("{} words", t.len());
        for w in ["casa", "Bello", "belle", "l'amico", "triste", "città"] {
            let m = t.lookup(w);
            println!("{w}: {} senses, first: {:?}", m.len(), m.first());
            assert!(!m.is_empty(), "{w}");
        }
    }

    #[test]
    fn only_italian_has_a_thesaurus() {
        install_italian(SAMPLE.as_bytes());
        assert!(for_lang(ProofLang::It).is_some_and(|t| !t.lookup("casa").is_empty()));
        assert!(for_lang(ProofLang::En).is_none());
        reload(); // keeps the installed one
        assert!(for_lang(ProofLang::It).is_some_and(|t| !t.is_empty()));
    }
}
