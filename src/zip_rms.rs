use failure::{bail, Fallible};
use std::{ffi::OsStr, fs::File, path::Path};
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub fn cli_unpack(input: impl AsRef<Path>, outdir: impl AsRef<Path>) -> Fallible<()> {
    let f = File::open(input)?;
    let mut zip = ZipArchive::new(f)?;
    std::fs::create_dir_all(outdir.as_ref())?;
    for index in 0..zip.len() {
        let mut file = zip.by_index(index)?;
        let mut outfile = File::create(outdir.as_ref().join(file.name()))?;
        std::io::copy(&mut file, &mut outfile)?;
    }
    Ok(())
}

pub fn cli_pack(indir: impl AsRef<Path>, output: impl AsRef<Path>) -> Fallible<()> {
    let mut files = vec![];

    let mut saw_rms = false;
    for entry in std::fs::read_dir(indir)? {
        let path = entry?.path();
        if path.extension() == Some(OsStr::new(".rms")) {
            if saw_rms {
                bail!("multiple .rms files found--only one is allowed per ZR@ map");
            }
            saw_rms = true;
        }
        if path.is_file() {
            files.push(path);
        }
    }

    let f = File::create(output)?;
    let mut zip = ZipWriter::new(f);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);

    for path in files {
        let name = match path.file_name() {
            Some(n) => n.to_string_lossy(),
            None => bail!("file without a file name?"),
        };
        zip.start_file(name, options)?;
        std::io::copy(&mut File::open(path)?, &mut zip)?;
    }

    Ok(())
}
