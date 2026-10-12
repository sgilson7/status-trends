//! The reader-visible text in `data/strings.json`: the page's own labels,
//! and the vocabularies that name the items and status groups (requests in
//! a tracker, or assignments in a class). The core checks the file before
//! the page draws anything from it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::status::Group;

/// The words for one kind of data.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Vocabulary {
    pub key: String,
    /// What the choice is called in the page's menu.
    pub name: String,
    /// One item and several: "request", "requests".
    pub item: String,
    pub items: String,
    /// The count of items that came in: "Requests received".
    pub received: String,
    pub requested: String,
    pub pending: String,
    pub done: String,
    pub excluded: String,
    /// What the group column holds: "team", "student".
    pub group: String,
}

impl Vocabulary {
    pub fn group_label(&self, g: Group) -> &str {
        match g {
            Group::Requested => &self.requested,
            Group::Pending => &self.pending,
            Group::Done => &self.done,
            Group::Excluded => &self.excluded,
        }
    }

    fn fields(&self) -> [(&'static str, &str); 10] {
        [
            ("key", &self.key),
            ("name", &self.name),
            ("item", &self.item),
            ("items", &self.items),
            ("received", &self.received),
            ("requested", &self.requested),
            ("pending", &self.pending),
            ("done", &self.done),
            ("excluded", &self.excluded),
            ("group", &self.group),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    /// Labels the page draws, by name.
    pub ui: BTreeMap<String, String>,
    /// The first vocabulary is the default.
    pub vocabularies: Vec<Vocabulary>,
}

impl Strings {
    /// The vocabulary with this key, or the first one.
    pub fn vocabulary(&self, key: &str) -> &Vocabulary {
        self.vocabularies
            .iter()
            .find(|v| v.key == key)
            .unwrap_or(&self.vocabularies[0])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringsError {
    NotJson(String),
    EmptyLabel(String),
    NoVocabulary,
    EmptyWord {
        vocabulary: usize,
        field: &'static str,
    },
}

impl StringsError {
    pub fn message(&self) -> String {
        match self {
            StringsError::NotJson(detail) => {
                format!("The strings file could not be read: {detail}")
            }
            StringsError::EmptyLabel(name) => {
                format!("The strings file has an empty label \"{name}\".")
            }
            StringsError::NoVocabulary => "The strings file has no vocabulary.".to_string(),
            StringsError::EmptyWord { vocabulary, field } => format!(
                "Vocabulary {} in the strings file has an empty \"{field}\".",
                vocabulary + 1
            ),
        }
    }
}

/// Parses and checks the strings file. Text that is only whitespace counts
/// as empty, because the page would draw nothing.
pub fn parse(json: &str) -> Result<Strings, StringsError> {
    let strings: Strings =
        serde_json::from_str(json).map_err(|e| StringsError::NotJson(e.to_string()))?;
    if let Some((name, _)) = strings.ui.iter().find(|(_, v)| v.trim().is_empty()) {
        return Err(StringsError::EmptyLabel(name.clone()));
    }
    if strings.vocabularies.is_empty() {
        return Err(StringsError::NoVocabulary);
    }
    for (i, v) in strings.vocabularies.iter().enumerate() {
        if let Some((field, _)) = v.fields().into_iter().find(|(_, t)| t.trim().is_empty()) {
            return Err(StringsError::EmptyWord {
                vocabulary: i,
                field,
            });
        }
    }
    Ok(strings)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn vocab_json(key: &str) -> String {
        format!(
            r#"{{"key": "{key}", "name": "N", "item": "request", "items": "requests", "received": "Requests received", "requested": "Still requested", "pending": "Pending", "done": "Completed", "excluded": "Excluded", "group": "team"}}"#
        )
    }

    pub fn sample() -> Strings {
        parse(&format!(
            r#"{{"ui": {{"title": "T"}}, "vocabularies": [{}, {}]}}"#,
            vocab_json("requests"),
            vocab_json("class")
        ))
        .unwrap()
    }

    #[test]
    fn the_shipped_file_is_accepted() {
        let s = parse(include_str!("../../../data/strings.json")).unwrap();
        assert!(s.ui.contains_key("title"));
        assert_eq!(s.vocabularies[0].key, "requests");
        assert_eq!(s.vocabulary("class").key, "class");
    }

    #[test]
    fn an_unknown_vocabulary_falls_back_to_the_first() {
        assert_eq!(sample().vocabulary("nope").key, "requests");
    }

    #[test]
    fn empty_text_is_refused_by_name() {
        let e = parse(r#"{"ui": {"title": " "}, "vocabularies": []}"#).unwrap_err();
        assert_eq!(
            e.message(),
            "The strings file has an empty label \"title\"."
        );
        let e = parse(r#"{"ui": {}, "vocabularies": []}"#).unwrap_err();
        assert_eq!(e, StringsError::NoVocabulary);
        let bad = vocab_json("x").replace("\"team\"", "\"\"");
        let e = parse(&format!(r#"{{"ui": {{}}, "vocabularies": [{bad}]}}"#)).unwrap_err();
        assert_eq!(
            e.message(),
            "Vocabulary 1 in the strings file has an empty \"group\"."
        );
        assert!(matches!(parse("{"), Err(StringsError::NotJson(_))));
    }
}
