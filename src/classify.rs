//! File classification by extension into categories.

use std::collections::HashMap;
use std::os::unix::ffi::OsStrExt;

use crate::tree::{Kind, NodeId, Tree};

/// Index of a category in a [`Classifier`].
pub type CatId = u8;

/// Built-in category keys, in display order. `other` is always last.
pub const BUILTIN: &[(&str, &[&str])] = &[
    (
        "video",
        &[
            "mp4", "mkv", "avi", "mov", "webm", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp", "m2ts",
        ],
    ),
    (
        "image",
        &[
            "jpg", "jpeg", "png", "gif", "bmp", "webp", "tif", "tiff", "svg", "heic", "heif",
            "avif", "raw", "cr2", "nef", "arw", "dng", "psd", "xcf", "ico",
        ],
    ),
    (
        "audio",
        &[
            "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "aiff", "alac", "mid",
        ],
    ),
    (
        "documents",
        &[
            "pdf", "doc", "docx", "odt", "rtf", "txt", "md", "xls", "xlsx", "ods", "csv", "ppt",
            "pptx", "odp", "epub", "mobi", "tex", "djvu",
        ],
    ),
    (
        "archives",
        &[
            "zip", "tar", "gz", "tgz", "bz2", "xz", "zst", "7z", "rar", "iso", "img", "dmg", "deb",
            "rpm", "apk", "jar", "cab", "lz4", "lzma",
        ],
    ),
    (
        "code",
        &[
            "rs", "c", "h", "cpp", "hpp", "cc", "py", "js", "ts", "jsx", "tsx", "go", "java", "kt",
            "rb", "php", "cs", "swift", "sh", "fish", "lua", "html", "css", "scss", "json", "toml",
            "yaml", "yml", "xml", "sql", "vue", "zig", "hs", "ex", "exs",
        ],
    ),
    (
        "cache",
        &[
            "tmp",
            "temp",
            "cache",
            "bak",
            "old",
            "swp",
            "swo",
            "part",
            "crdownload",
            "log",
            "pyc",
            "o",
            "obj",
            "class",
        ],
    ),
];

/// Key of the fallback category.
pub const OTHER: &str = "other";

/// Maps extensions to categories.
#[derive(Debug, Clone)]
pub struct Classifier {
    names: Vec<String>,
    by_ext: HashMap<Vec<u8>, CatId>,
}

impl Default for Classifier {
    fn default() -> Self {
        Self::new(&[])
    }
}

impl Classifier {
    /// Built-in categories, with `overrides` applied: an entry whose name is
    /// a built-in key replaces that list; any other name adds a category.
    pub fn new(overrides: &[(String, Vec<String>)]) -> Self {
        let mut lists: Vec<(String, Vec<String>)> = BUILTIN
            .iter()
            .map(|(k, exts)| {
                (
                    (*k).to_owned(),
                    exts.iter().map(|e| (*e).to_owned()).collect(),
                )
            })
            .collect();
        for (name, exts) in overrides {
            let name = name.to_ascii_lowercase();
            if name == OTHER {
                continue;
            }
            match lists.iter_mut().find(|(k, _)| *k == name) {
                Some(entry) => entry.1 = exts.clone(),
                None => lists.push((name, exts.clone())),
            }
        }
        let mut names = Vec::new();
        let mut by_ext = HashMap::new();
        for (i, (name, exts)) in lists.into_iter().enumerate() {
            names.push(name);
            for e in exts {
                let e = e.trim_start_matches('.').to_ascii_lowercase().into_bytes();
                // First definition wins so built-ins keep their order.
                by_ext.entry(e).or_insert(i as CatId);
            }
        }
        names.push(OTHER.to_owned());
        Classifier { names, by_ext }
    }

    /// Number of categories, `other` included.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Never true: `other` always exists.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Key of a category.
    pub fn name(&self, id: CatId) -> &str {
        self.names
            .get(usize::from(id))
            .map_or(OTHER, String::as_str)
    }

    /// Id of the `other` category.
    pub fn other(&self) -> CatId {
        (self.names.len() - 1) as CatId
    }

    /// Category of a file name (by its last extension, case-insensitive).
    pub fn of_name(&self, name: &[u8]) -> CatId {
        let Some(dot) = name.iter().rposition(|&b| b == b'.') else {
            return self.other();
        };
        if dot == 0 || dot + 1 == name.len() {
            return self.other();
        }
        let ext = name[dot + 1..].to_ascii_lowercase();
        self.by_ext
            .get(&ext)
            .copied()
            .unwrap_or_else(|| self.other())
    }

    /// Category of a node; directories and non-files fall in `other`.
    pub fn of_node(&self, tree: &Tree, id: NodeId) -> CatId {
        match tree.node(id).kind {
            Kind::File => self.of_name(tree.name(id).as_bytes()),
            _ => self.other(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension_case_insensitive() {
        let c = Classifier::default();
        assert_eq!(c.name(c.of_name(b"movie.MKV")), "video");
        assert_eq!(c.name(c.of_name(b"a.tar.gz")), "archives");
        assert_eq!(c.name(c.of_name(b"photo.jpeg")), "image");
        assert_eq!(c.name(c.of_name(b"main.rs")), "code");
        assert_eq!(c.name(c.of_name(b"x.tmp")), "cache");
        assert_eq!(c.name(c.of_name(b"Makefile")), "other");
        assert_eq!(c.name(c.of_name(b".bashrc")), "other");
        assert_eq!(c.name(c.of_name(b"trailing.")), "other");
    }

    #[test]
    fn overrides_replace_and_extend() {
        let c = Classifier::new(&[
            ("video".into(), vec!["mp4".into()]),
            ("models".into(), vec![".gguf".into(), "safetensors".into()]),
        ]);
        assert_eq!(
            c.name(c.of_name(b"a.mkv")),
            "other",
            "mkv removed from video"
        );
        assert_eq!(c.name(c.of_name(b"a.mp4")), "video");
        assert_eq!(c.name(c.of_name(b"llm.GGUF")), "models");
        assert_eq!(c.name(c.other()), "other");
        assert_eq!(c.len(), BUILTIN.len() + 2);
    }
}
