use crate::error::{Error, Result};
use smartstring::alias::String;
use std::{collections::HashMap, iter::FromIterator, ops::Deref, path::Path};
use unicase::UniCase;

pub struct LanguagesToEntries(HashMap<LanguageCode, Vec<UniCase<String>>>);

impl LanguagesToEntries {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn push(&mut self, language_code: LanguageCode, title: &str) {
        self.0
            .entry(language_code)
            .or_insert_with(Vec::new)
            .push(UniCase::new(String::from(title)));
    }
}

impl IntoIterator for LanguagesToEntries {
    type Item =
        <HashMap<LanguageCode, Vec<UniCase<String>>> as IntoIterator>::Item;
    type IntoIter =
        <HashMap<LanguageCode, Vec<UniCase<String>>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct LanguageCode {
    data: [u8; LanguageCode::MAX],
    len: u8,
}

impl LanguageCode {
    const MAX: usize = "aaa-aaa-aaa".len();

    fn new(code: &str) -> Option<Self> {
        if code.len() > Self::MAX {
            None
        } else {
            let mut data = [0u8; Self::MAX];
            &mut data[..code.len()].copy_from_slice(code.as_bytes());
            let len = code.len() as u8;
            Some(Self { data, len })
        }
    }
}

impl Deref for LanguageCode {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe {
            std::str::from_utf8_unchecked(&self.data[..self.len as usize])
        }
    }
}

impl AsRef<Path> for LanguageCode {
    fn as_ref(&self) -> &Path {
        self.deref().as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageNameToCode(HashMap<String, LanguageCode>);

impl LanguageNameToCode {
    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&LanguageCode> {
        self.0.get(key.as_ref())
    }

    pub fn from_tsv_file(path: &Path) -> Result<LanguageNameToCode> {
        let language_name_to_code = std::fs::read_to_string(path)
            .map_err(|e| Error::from_io(e, "read from", path))?;
        language_name_to_code
            .lines()
            .enumerate()
            .filter(|(_, line)| *line != "")
            .map(|(i, line)| {
                let mut splitter = line.splitn(2, '\t');
                let (first, second) = (splitter.next(), splitter.next());
                if let (Some(name), Some(code)) = (first, second) {
                    let code = LanguageCode::new(code).ok_or_else(|| {
                        Error::InvalidLanguageCode {
                            path: path.into(),
                            line: line.into(),
                            line_number: i + 1,
                        }
                    })?;
                    Ok((name.into(), code))
                } else {
                    Err(Error::InvalidNameToCodeFormat {
                        path: path.into(),
                        line: line.into(),
                        line_number: i + 1,
                    })
                }
            })
            .collect()
    }
}

impl FromIterator<(String, LanguageCode)> for LanguageNameToCode {
    fn from_iter<T: IntoIterator<Item = (String, LanguageCode)>>(
        iter: T,
    ) -> Self {
        Self(FromIterator::from_iter(iter))
    }
}
