use serde::{Deserialize, Serialize};
use std::fs::{File, Metadata};
use std::io::{self, BufReader, BufWriter, Seek};
use std::time::Instant;
use std::path::Path;


#[derive(Debug)]
pub enum SerChoice {
    JSON,
    YAML
}


#[derive(Debug)]
pub struct FileOptions {
    read: bool,
    write: bool,
    create: bool,
}

#[derive(Debug)]
pub struct FileIO<S: AsRef<Path>> {
    pub filename: S,
    pub options: FileOptions,
    buf_size: usize,
}

impl FileOptions {
    pub fn new(read: bool, write: bool, create: bool) -> Self {
        FileOptions { read, write, create }
    }
}

impl<S: AsRef<Path>> std::io::Write for FileIO<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }

    fn by_ref(&mut self) -> &mut Self
        where
            Self: Sized, {
        todo!()
    }

    fn write_all(&mut self, mut buf: &[u8]) -> io::Result<()> {
        todo!()
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> io::Result<()> {
        todo!()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        todo!()
    }
}

impl<S: AsRef<Path>> FileIO<S> {
    pub fn new(filename: S, options: FileOptions) -> Self {
        Self {
            filename,
            options,
            buf_size: 8192usize, // Default capacity for BufRead/Writer
        }
    }

    // pub fn create_with_options(&mut self,
    pub fn with_buf_size<N: Into<usize>>(&mut self, buf_size: N) {
        self.buf_size = buf_size.into();
    }

    pub fn get_buf_size(&self) -> usize {
        self.buf_size
    }

    pub fn set_buf_size<N: Into<usize>>(&mut self, buf_size: N) {
        self.with_buf_size(buf_size);
    }

    /// # Errors
    /// See [`std::fs::File::metadata`].
    pub fn metadata(&self, f: &File) -> io::Result<Metadata> {
        f.metadata()
    }
    pub fn exists(&self, f: &File) -> bool {
        self.metadata(f).is_ok()
    }

    pub fn set_file_path(&mut self, fp: S) {
        self.filename = fp;
    }
    // Is this the best way?
    fn file(&self) -> File {
        let options = &self.options;
        let filename = &self.filename;

        std::fs::OpenOptions::new()
            .read(options.read)
            .write(options.write)
            .create(options.create)
            .open(filename)
            .expect("Failed to deserialize file contents")
    }

    /// # Errors
    /// Errors if `Self::rewind` fails.
    pub fn open_file(&self) -> Result<File, std::io::Error> {
        let mut file = self.file(); // Open once

        if !self.exists(&file) {
            return Err(io::ErrorKind::NotFound.into());
        };

        self.rewind(&mut file);
        Ok(file)
    }

    fn rewind(&self, file: &mut File) {
        // Need to rewind cursor just in case this isn't first operation
        let curr_pos = file.stream_position().expect("Error seeking file cursor");

        if curr_pos == 0 {
            return; // Early exit. Don't rewind if not needed.
        }

        file.rewind().expect("Erorr rewinding cursor")
    }

    pub fn has_data(&self, f: &File) -> bool {
        self.metadata(f)
            .and_then(|m| Ok(m.len() > 0))
            .expect("Could not read metadata")
    }

    pub fn get_writer(&mut self) -> Result<BufWriter<File>, std::io::Error> {
        let file = self.open_file()?;

        Ok(BufWriter::with_capacity(self.get_buf_size(), file))
    }

    pub fn get_reader(&self) -> Result<BufReader<File>, std::io::Error> {
        let file = self.open_file()?;

        // TODO: Is file.read_to_string(...); faster?
        Ok(BufReader::with_capacity(self.get_buf_size(), file)) // Speedy
    }

    /// # Errors
    /// When file does not exist or serialization fails.
    pub fn write<J: Serialize>(
        &mut self,
        data: &J,
    ) -> Result<(), std::io::Error> {
        let buffer = self.get_writer()?;
        let mut serialiser = serde_yml::ser::Serializer::new(buffer);

        // DEBUG
        let now = Instant::now();
        data.serialize(&mut serialiser).expect("Failed to write to file");
        println!("Wrote file in {:?}", now.elapsed()); // DEBUG

        Ok(())
    }

    /// # Errors
    /// Errors when the file does not exist or deserialization fails.
    pub fn read<'de, T: Deserialize<'de>>(&self, ser: SerChoice) -> Result<T, std::io::Error> {
        let buffer = self.get_reader()?;

        let t = match ser {
            SerChoice::JSON => {
                let mut deserialiser = serde_json::de::Deserializer::from_reader(buffer);
                T::deserialize(&mut deserialiser).expect("Failed to read from file")
            },
            SerChoice::YAML => {
                let deserialiser = serde_yml::de::Deserializer::from_reader(buffer);
                T::deserialize(deserialiser).expect("Failed to read from file")
            }
        };


        Ok(t)
    }

    pub fn clear_contents(&mut self) -> Result<(), std::io::Error> {
        let file = self.open_file()?;

        file.set_len(0)?;
        file.sync_all()?;
        Ok(())
    }
}
