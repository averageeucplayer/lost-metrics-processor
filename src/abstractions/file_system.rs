use std::{cell::RefCell, env, fs::File, io, path::{Path, PathBuf}, rc::Rc};

use anyhow::{Ok, Result};
use hashbrown::HashMap;

pub trait FileSystem {
    type Reader: io::Read;
    type Writer: io::Write;

    fn exists(&self, path: &Path) -> bool;
    fn get_writer(&mut self, path: &Path) -> Result<Self::Writer>;
    fn get_reader(&mut self, path: &Path) -> Result<Self::Reader>;
    fn get_executable_directory(&self) -> Result<PathBuf>;
}

pub struct DefaultFileSystem;

impl FileSystem for DefaultFileSystem {
    type Reader = File;
    type Writer = File;

    fn get_writer(&mut self, path: &Path) -> Result<Self::Writer> {
        File::create(path).map_err(|e| anyhow::anyhow!("Could not create file: {}", e))
    }

    fn get_reader(&mut self, path: &Path) -> Result<Self::Reader> {
        File::open(path).map_err(|e| anyhow::anyhow!("Could not open file: {}", e))
    }

    fn get_executable_directory(&self) -> Result<PathBuf> {
        let executable_path = if cfg!(test) {
            env::current_exe()?.parent().unwrap().to_path_buf()
        } else {
            env::current_exe()?
        };

        let executable_directory = executable_path.parent().unwrap();
        let path = executable_directory.to_path_buf();
        
        Ok(path)
    }
    
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

impl DefaultFileSystem {
    pub fn new() -> Self {
        Self
    }
}

pub struct MemoryFileSystem {
    map: HashMap<PathBuf, MemoryFileSystemEntry>
}

impl FileSystem for MemoryFileSystem {
    type Reader = MemoryFileSystemEntry;
    type Writer = MemoryFileSystemEntry;

    fn get_writer(&mut self, path: &Path) -> Result<Self::Writer> {
        let entry = self.map.entry(path.to_path_buf())
            .or_insert_with(MemoryFileSystemEntry::new);

        Ok(entry.clone())
    }

    fn get_reader(&mut self, path: &Path) -> Result<Self::Reader> {
        self.map.get(path)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("File not found: {:?}", path))
    }

    fn get_executable_directory(&self) -> Result<PathBuf> {
        Ok(std::env::temp_dir())
    }
    
    fn exists(&self, path: &Path) -> bool {
        self.map.contains_key(path)
    }
}

impl MemoryFileSystem {
    pub fn new() -> Self {
        Self {
            map: HashMap::new()
        }
    }
}

pub struct MemoryFileSystemEntry {
    data: Rc<RefCell<Vec<u8>>>,
    position: usize,
}

impl Clone for MemoryFileSystemEntry {  
    fn clone(&self) -> Self {  
        Self {  
            data: self.data.clone(),
            position: self.position,
        }  
    }  
} 

impl io::Read for MemoryFileSystemEntry {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = self.data.borrow();
        let available = data.len().saturating_sub(self.position);
        let bytes_to_read = available.min(buf.len());

        if bytes_to_read == 0 {
            return io::Result::Ok(0);
        }

        buf[..bytes_to_read].copy_from_slice(&data[self.position..self.position + bytes_to_read]);
        self.position += bytes_to_read;

        io::Result::Ok(bytes_to_read)
    }
}

impl io::Write for MemoryFileSystemEntry {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut data = self.data.borrow_mut();
        data.extend_from_slice(buf);
        io::Result::Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Result::Ok(())
    }
}

impl MemoryFileSystemEntry {
    pub fn new() -> Self {
        Self {
            data: Rc::new(RefCell::new(Vec::new())),
            position: 0
        }
    }
}