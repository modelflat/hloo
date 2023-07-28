use core::slice;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Seek, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
};

use memmap2::{MmapMut, MmapOptions};
use thiserror::Error;

use crate::util::partition;

#[derive(Debug, Error)]
pub enum MmVecError {
    #[error("signature does not match: expected: {expected}, got: {actual} ")]
    SignatureMismatch { expected: u64, actual: u64 },
    #[error("loading vectors which are not fully initialized or have trailing data in the file is not supported!")]
    UninitializedVectorLoad {},
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct MmVec<T> {
    data: Data<T>,
    path: PathBuf,
}

impl<T> MmVec<T>
where
    T: Copy,
{
    fn new(data: Data<T>, path: PathBuf) -> Self {
        Self { data, path }
    }

    /// Creates an uninitialized vector with given length.
    unsafe fn with_length_uninit(sig: u64, len: usize, path: PathBuf) -> Result<Self, MmVecError> {
        let file = create_new_file(&path)?;
        // SAFETY: we just created this file
        let data = unsafe { Data::new_uninit(file, sig, len as u64)? };
        Ok(Self::new(data, path))
    }

    pub fn new_empty(sig: u64, path: PathBuf) -> Result<Self, MmVecError> {
        unsafe { Self::with_length_uninit(sig, 0, path) }
    }

    /// Dumps a regular in-memory vector into path, then mmaps it.
    pub fn from_vec(sig: u64, vec: Vec<T>, path: PathBuf) -> Result<Self, MmVecError> {
        let file = create_new_file(&path)?;
        // SAFETY: we just created this file
        let data = unsafe { Data::new_with_data(file, sig, &vec)? };
        Ok(Self::new(data, path))
    }

    /// Try to create a vector from the given path. Panic if the signature does not match
    pub fn from_path(sig: u64, path: PathBuf) -> Result<Self, MmVecError> {
        let file = open_file(&path)?;
        // SAFETY: we are going to check the data
        let data = unsafe {
            let data = Data::<T>::from_file_unchecked(file)?;
            if data.sig() != sig {
                return Err(MmVecError::SignatureMismatch {
                    expected: sig,
                    actual: data.sig(),
                });
            }
            // only whole-file, fully initialized vectors are supported
            if data.len() != data.capacity() as u64 {
                return Err(MmVecError::UninitializedVectorLoad {});
            }
            data
        };
        Ok(Self::new(data, path))
    }

    /// Returns a path to the backing file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns length of the vector.
    #[must_use]
    pub fn len(&self) -> usize {
        unsafe { self.data.len() as usize }
    }

    /// Returns the signature of the vector.
    #[must_use]
    pub fn sig(&self) -> u64 {
        unsafe { self.data.sig() }
    }

    /// Whether this vector is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that file has not been tampered with.
    #[must_use]
    pub unsafe fn as_slice(&self) -> &[T] {
        self.data.as_slice()
    }

    /// # Safety
    /// This is unsafe because we can't guarantee that file has not been tampered with.
    #[must_use]
    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        self.data.as_slice_mut()
    }

    /// Flushes memory-mapped data into file.
    pub fn flush(&self) -> Result<(), MmVecError> {
        self.data.flush()?;
        Ok(())
    }

    /// Destroys self, removing the underlying file
    pub fn destroy(self) -> Result<(), MmVecError> {
        let path = self.path.clone();
        drop(self);
        fs::remove_file(path)?;
        Ok(())
    }

    /// Copies self into path, and returns a new vector at this path.
    pub fn copy_to(&self, path: PathBuf) -> Result<Self, MmVecError> {
        self.flush()?;
        fs::copy(&self.path, &path)?;

        let copied = open_file(&path)?;
        // SAFETY: we just created this file from a valid vector
        let copied_data = unsafe { Data::from_file_unchecked(copied)? };
        Ok(Self::new(copied_data, path))
    }

    /// Moves self into path, and returns a new vector at this path.
    pub fn move_to(self, path: PathBuf) -> Result<Self, MmVecError> {
        self.flush()?;
        fs::rename(&self.path, &path)?;
        drop(self);

        let moved = open_file(&path)?;
        // SAFETY: we just created this file from a valid vector
        let moved_data = unsafe { Data::from_file_unchecked(moved)? };
        Ok(Self::new(moved_data, path))
    }

    pub fn insert_sorted<O, F>(&mut self, items: &[T], sort_key: F) -> Result<(), MmVecError>
    where
        F: Fn(&T) -> O,
        O: Ord,
    {
        let current_len = self.len();
        unsafe {
            self.data.resize(current_len + items.len())?;
            self.data.as_slice_mut()[current_len..].copy_from_slice(items);
            self.data.as_slice_mut().sort_unstable_by_key(sort_key);
        }
        Ok(())
    }

    pub fn remove_matching<O, F, S>(&mut self, predicate: F, sort_key: S) -> Result<(), MmVecError>
    where
        F: Fn(&T) -> bool,
        S: Fn(&T) -> O,
        O: Ord,
    {
        unsafe {
            let split = partition(self.data.as_slice_mut(), |el| !predicate(el));
            self.data.resize(split)?;
            self.data.as_slice_mut().sort_unstable_by_key(sort_key);
        }
        Ok(())
    }
}

/// Low-level memory-mapped data
struct Data<T> {
    file: File,
    header_mmap: MmapMut,
    data_mmap: MmapMut,
    dummy: PhantomData<T>,
}

impl<T> Data<T> {
    const HEADER_SIZE: u64 = 16;

    unsafe fn header_offset(&self, offset: usize) -> *const u8 {
        assert!(offset < Self::HEADER_SIZE as usize, "offset is out of bounds");
        (self.header_mmap.as_ptr() as *const u8).add(offset)
    }

    pub unsafe fn sig(&self) -> u64 {
        *(self.header_offset(0) as *const u64)
    }

    unsafe fn set_sig(&mut self, sig: u64) {
        *(self.header_offset(0) as *mut u64) = sig;
    }

    pub unsafe fn len(&self) -> u64 {
        *(self.header_offset(8) as *const u64)
    }

    unsafe fn set_len(&mut self, len: u64) {
        *(self.header_offset(8) as *mut u64) = len;
    }

    pub fn capacity(&self) -> usize {
        self.data_mmap.len() / std::mem::size_of::<T>()
    }

    pub unsafe fn as_ptr(&self) -> *const T {
        self.data_mmap.as_ptr() as *const T
    }

    pub unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self.data_mmap.as_mut_ptr() as *mut T
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        slice::from_raw_parts(self.as_ptr(), self.len() as usize)
    }

    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        slice::from_raw_parts_mut(self.as_mut_ptr(), self.len() as usize)
    }

    pub unsafe fn from_file_unchecked(file: File) -> io::Result<Self> {
        let len_bytes = file.metadata()?.len();
        let header_mmap = mmap(&file, 0, Self::HEADER_SIZE as usize)?;
        let data_mmap = mmap(&file, Self::HEADER_SIZE, (len_bytes - Self::HEADER_SIZE) as usize)?;
        Ok(Self {
            file,
            header_mmap,
            data_mmap,
            dummy: PhantomData,
        })
    }

    pub unsafe fn new_uninit(file: File, sig: u64, len: u64) -> io::Result<Self> {
        let needed_bytes = std::mem::size_of::<T>() as u64 * len;
        file.set_len(Self::HEADER_SIZE + needed_bytes)?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_sig(sig);
        data.set_len(len);
        data.header_mmap.flush()?;
        Ok(data)
    }

    pub unsafe fn new_with_data(mut file: File, sig: u64, data: &[T]) -> io::Result<Self> {
        let len = data.len();
        let bytes = slice_as_bytes(data);
        file.set_len(Self::HEADER_SIZE + bytes.len() as u64)?;
        file.seek(io::SeekFrom::Start(Self::HEADER_SIZE))?;
        file.write_all(bytes)?;
        file.flush()?;
        let mut data = Self::from_file_unchecked(file)?;
        data.set_sig(sig);
        data.set_len(len as u64);
        data.header_mmap.flush()?;
        Ok(data)
    }

    pub unsafe fn resize(&mut self, len: usize) -> io::Result<()> {
        let new_len_bytes = len * std::mem::size_of::<T>();
        self.data_mmap.flush()?;
        self.file.set_len(Self::HEADER_SIZE + new_len_bytes as u64)?;
        self.data_mmap = mmap(&self.file, Self::HEADER_SIZE, new_len_bytes)?;
        self.set_len(len as u64);
        Ok(())
    }

    pub fn flush(&self) -> io::Result<()> {
        self.header_mmap.flush()?;
        self.data_mmap.flush()?;
        Ok(())
    }
}

impl<T> Drop for Data<T> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

fn create_new_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(path)
}

fn open_file(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(false)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
}

fn slice_as_bytes<T>(data: &[T]) -> &[u8] {
    let len_bytes = std::mem::size_of_val(data);
    // SAFETY: this is a valid slice.
    unsafe { slice::from_raw_parts(data.as_ptr() as *const u8, len_bytes) }
}

unsafe fn mmap(file: &File, offset: u64, len: usize) -> io::Result<MmapMut> {
    let mut opts = MmapOptions::new();
    let mmap = opts.offset(offset).len(len).map_mut(file)?;
    #[cfg(unix)]
    {
        mmap.advise(memmap2::Advice::Random).ok();
    }
    Ok(mmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_header_fields() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        let file = create_new_file(&test_path).unwrap();

        unsafe {
            let mut data = Data::<u64>::new_uninit(file, 42, 100).expect("failed to create data");
            assert_eq!(data.sig(), 42, "sig should be 42!");
            assert_eq!(data.len(), 100, "len should be 100!");
            data.set_sig(129);
            assert_eq!(data.sig(), 129, "data sig cannot be set correctly!");

            drop(data);

            let file = open_file(&test_path).unwrap();
            let data = Data::<u64>::from_file_unchecked(file).expect("failed to create data");
            assert_eq!(data.sig(), 129, "sig is not preserved!");
            assert_eq!(data.len(), 100, "len is not preserved!");
        }
    }

    #[test]
    fn test_data_resize_extend() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        let file = create_new_file(&test_path).unwrap();
        unsafe {
            let mut data = Data::<u64>::new_uninit(file, 42, 100).unwrap();
            assert_eq!(data.len(), 100, "init len should be 100");
            data.resize(1000).unwrap();
            assert_eq!(data.len(), 1000, "updated len should be 1000");
            assert_eq!(
                data.data_mmap.len(),
                1000 * std::mem::size_of::<u64>(),
                "mmap size should correspond to the resized data"
            );
            let file2 = open_file(&test_path).unwrap();
            assert_eq!(
                file2.metadata().unwrap().len(),
                Data::<u64>::HEADER_SIZE + 1000 * std::mem::size_of::<u64>() as u64,
                "file should have corresponding length"
            );
        }
        let file3 = open_file(&test_path).unwrap();
        assert_eq!(
            file3.metadata().unwrap().len(),
            Data::<u64>::HEADER_SIZE + 1000 * std::mem::size_of::<u64>() as u64,
            "file should have corresponding length after data is closed"
        );
    }

    #[test]
    fn test_data_resize_shrink() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        let file = create_new_file(&test_path).unwrap();
        unsafe {
            let mut data = Data::<u64>::new_uninit(file, 42, 100).unwrap();
            assert_eq!(data.len(), 100, "init len should be 100");
            data.resize(10).unwrap();
            assert_eq!(data.len(), 10, "updated len should be 10");
            assert_eq!(
                data.data_mmap.len(),
                10 * std::mem::size_of::<u64>(),
                "mmap size should correspond to the resized data"
            );
            let file2 = open_file(&test_path).unwrap();
            assert_eq!(
                file2.metadata().unwrap().len(),
                Data::<u64>::HEADER_SIZE + 10 * std::mem::size_of::<u64>() as u64,
                "file should have corresponding length"
            );
        }
        let file3 = open_file(&test_path).unwrap();
        assert_eq!(
            file3.metadata().unwrap().len(),
            Data::<u64>::HEADER_SIZE + 10 * std::mem::size_of::<u64>() as u64,
            "file should have corresponding length after data is closed"
        );
    }

    #[test]
    fn test_from_vec_to_vec() {
        let tmp = tempfile::tempdir().unwrap();
        let test_path = tmp.path().join("test.mmvec");
        let data = vec![(199, 200), (200, 532), (449, 400)];
        let vec = MmVec::from_vec(0, data.clone(), test_path.clone()).expect("failed to create memvec");
        drop(vec);
        let result = MmVec::<(i32, i32)>::from_path(0, test_path).expect("failed to load memvec from file");
        assert_eq!(unsafe { result.as_slice() }, data.as_slice(), "data was corrupted");
    }
}
