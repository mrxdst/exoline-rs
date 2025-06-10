use unicase::UniCase;

use super::{controller::FileSetInternal, File};

/// A collection of files.
pub struct FileSet {
    pub(crate) file_sets: Vec<FileSetInternal>,
}

impl FileSet {
    /// Retrieves a file in the collection.
    pub fn get(&self, filename: &str) -> Option<File> {
        let file_key = UniCase::new(filename.into());
        for file_set in self.file_sets.iter() {
            if let Some((file_key, (load_number, file))) = file_set.get_key_value(&file_key) {
                return Some(File {
                    file: file.clone(),
                    file_key: file_key.clone(),
                    load_number: *load_number,
                });
            }
        }

        return None;
    }

    /// Returns the number of files in the collection.
    pub fn len(&self) -> usize {
        self.file_sets.iter().map(|f| f.len()).reduce(|acc, e| acc + e).unwrap_or_default()
    }

    /// An iterator over all the files in the collection.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = File> + Send + 'a {
        self.file_sets.iter().flat_map(|f| f.iter()).map(|(file_key, (load_number, file))| File {
            file: file.clone(),
            file_key: file_key.clone(),
            load_number: *load_number,
        })
    }
}
