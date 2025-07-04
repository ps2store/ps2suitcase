use crate::data::virtual_file::VirtualFile;
use std::ffi::OsStr;
use std::ops::Index;
use std::path::Path;
use crate::io::calculate_size::calculate_size;

#[derive(Default)]
pub struct Files(pub Vec<VirtualFile>, u64);

impl Files {
    pub fn from(files: Vec<VirtualFile>) -> std::io::Result<Self> {
        let mut slf = Self(files.to_vec(), 0);
        slf.calculate_size()?;
        slf.sort();

        Ok(slf)
    }
    pub fn add_file<P: AsRef<Path>>(&mut self, file_path: P) -> std::io::Result<()> {
        let name = file_path
            .as_ref()
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidFilename,
                "Invalid file name",
            ))?
            .to_string();
        let size = std::fs::metadata(&file_path)?.len();

        self.0.push(VirtualFile {
            name,
            file_path: file_path.as_ref().into(),
            size,
        });
        self.calculate_size()?;

        Ok(())
    }

    fn sort(&mut self) {
        self.0.sort_by(|a, b| a.name.partial_cmp(&b.name).unwrap());
    }

    fn calculate_size(&mut self) -> std::io::Result<()> {
        self.1 = calculate_size(&self.0)?;

        Ok(())
    }

    pub fn calculated_size(&self) -> u64 {
        self.1
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &VirtualFile> {
        self.0.iter()
    }
}

impl Index<usize> for Files {
    type Output = VirtualFile;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}