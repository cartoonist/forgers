use flate2::read::MultiGzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::error;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, BufWriter, Read, Stdin, Stdout, Write};
use std::path::Path;
use vcf::{VCFError, VCFHeader, VCFReader, VCFRecord, VCFWriter};

use crate::option::Opt;

pub enum StreamType {
    File,
    Stdio,
}

#[derive(Default)]
pub enum CompressionType {
    None,
    #[default] // default when compression is forced
    Gzip,
    Bgzip,
}

pub fn stream_type<T>(path: &T) -> StreamType
where
    T: AsRef<Path>,
{
    if path.as_ref() == Path::new("-") {
        StreamType::Stdio
    } else {
        StreamType::File
    }
}

pub fn compress_type<T>(path: &T, force: bool) -> CompressionType
where
    T: AsRef<Path>,
{
    let filename = path.as_ref().display().to_string();
    if force {
        CompressionType::default()
    } else if filename.ends_with(".gz") {
        CompressionType::Gzip
    } else if filename.ends_with(".bgz") {
        CompressionType::Bgzip
    } else {
        CompressionType::None
    }
}

pub fn path_or<T>(path: &T, stdio: &str) -> String
where
    T: AsRef<Path>,
{
    if path.as_ref() == Path::new("-") {
        stdio.to_string()
    } else {
        path.as_ref().display().to_string()
    }
}

pub trait Process<W: Write, R: Read> {
    fn process(&mut self, writer: VCFWriter<BufWriter<W>>, reader: VCFReader<BufReader<R>>);
}

pub fn launch_iostream(opt: Opt) {
    let ipath = opt.input.clone();
    let opath = opt.output.clone();
    load_istream(&ipath, &opath, opt)
}

fn load_istream<T>(ipath: &T, opath: &T, opt: Opt)
where
    T: AsRef<Path>,
{
    match stream_type(&ipath) {
        StreamType::Stdio => {
            let mut lstdin = stdin();
            if is_gzipped_stdin(&mut lstdin) {
                match reader_stdio_gz(lstdin) {
                    Ok(vcf_reader) => {
                        load_ostream(&opath, vcf_reader, opt);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &ipath.as_ref().display());
                        std::process::exit(1);
                    }
                }
            } else {
                match reader_stdio(lstdin) {
                    Ok(vcf_reader) => {
                        load_ostream(&opath, vcf_reader, opt);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &ipath.as_ref().display());
                        std::process::exit(1);
                    }
                }
            }
        }
        StreamType::File => {
            let gz = match is_gzipped_file(&ipath) {
                Ok(v) => v,
                Err(e) => {
                    error!("{}: '{}'", e, &ipath.as_ref().display());
                    std::process::exit(1);
                }
            };
            match compress_type(&ipath, gz) {
                CompressionType::Gzip | CompressionType::Bgzip => match reader_file_gz(&ipath) {
                    Ok(vcf_reader) => {
                        load_ostream(&opath, vcf_reader, opt);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &ipath.as_ref().display());
                        std::process::exit(1);
                    }
                },
                CompressionType::None => match reader_file(&ipath) {
                    Ok(vcf_reader) => {
                        load_ostream(&opath, vcf_reader, opt);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &ipath.as_ref().display());
                        std::process::exit(1);
                    }
                },
            }
        }
    }
}

fn load_ostream<T, R>(path: &T, vcf_reader: VCFReader<BufReader<R>>, mut opt: Opt)
where
    T: AsRef<Path>,
    R: Read,
{
    match stream_type(&path) {
        StreamType::Stdio => {
            if opt.gzip {
                match writer_stdio_gz(&vcf_reader.header()) {
                    Ok(vcf_writer) => {
                        opt.process(vcf_writer, vcf_reader);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &path.as_ref().display());
                        std::process::exit(1);
                    }
                }
            } else {
                match writer_stdio(&vcf_reader.header()) {
                    Ok(vcf_writer) => {
                        opt.process(vcf_writer, vcf_reader);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &path.as_ref().display());
                        std::process::exit(1);
                    }
                }
            }
        }
        StreamType::File => match compress_type(&path, opt.gzip) {
            CompressionType::Gzip | CompressionType::Bgzip => {
                match writer_file_gz(&path, &vcf_reader.header()) {
                    Ok(vcf_writer) => {
                        opt.process(vcf_writer, vcf_reader);
                    }
                    Err(e) => {
                        error!("{}: '{}'", e, &path.as_ref().display());
                        std::process::exit(1);
                    }
                }
            }
            CompressionType::None => match writer_file(&path, &vcf_reader.header()) {
                Ok(vcf_writer) => {
                    opt.process(vcf_writer, vcf_reader);
                }
                Err(e) => {
                    error!("{}: '{}'", e, &path.as_ref().display());
                    std::process::exit(1);
                }
            },
        },
    }
}

pub fn writer_file<T>(path: &T, header: &VCFHeader) -> Result<VCFWriter<BufWriter<File>>, VCFError>
where
    T: AsRef<Path>,
{
    let file = File::create(path)?;
    VCFWriter::new(BufWriter::new(file), header)
}

pub fn writer_file_gz<T>(
    path: &T,
    header: &VCFHeader,
) -> Result<VCFWriter<BufWriter<GzEncoder<File>>>, VCFError>
where
    T: AsRef<Path>,
{
    let file = File::create(path)?;
    VCFWriter::new(
        BufWriter::new(GzEncoder::new(file, Compression::default())),
        header,
    )
}

pub fn writer_stdio(header: &VCFHeader) -> Result<VCFWriter<BufWriter<Stdout>>, VCFError> {
    VCFWriter::new(BufWriter::new(stdout()), header)
}

pub fn writer_stdio_gz(
    header: &VCFHeader,
) -> Result<VCFWriter<BufWriter<GzEncoder<Stdout>>>, VCFError> {
    VCFWriter::new(
        BufWriter::new(GzEncoder::new(stdout(), Compression::default())),
        header,
    )
}

pub fn reader_file<T>(path: &T) -> Result<VCFReader<BufReader<File>>, VCFError>
where
    T: AsRef<Path>,
{
    let file = File::open(path)?;
    VCFReader::new(BufReader::new(file))
}

pub fn reader_file_gz<T>(path: &T) -> Result<VCFReader<BufReader<MultiGzDecoder<File>>>, VCFError>
where
    T: AsRef<Path>,
{
    let file = File::open(path)?;
    VCFReader::new(BufReader::new(MultiGzDecoder::new(file)))
}

pub fn reader_stdio(lstdin: Stdin) -> Result<VCFReader<BufReader<Stdin>>, VCFError> {
    VCFReader::new(BufReader::new(lstdin))
}

pub fn reader_stdio_gz(
    lstdin: Stdin,
) -> Result<VCFReader<BufReader<MultiGzDecoder<Stdin>>>, VCFError> {
    VCFReader::new(BufReader::new(MultiGzDecoder::new(lstdin)))
}

pub fn nof_records<R>(vcf_reader: &mut VCFReader<BufReader<R>>) -> Result<usize, VCFError>
where
    R: Read,
{
    // prepare VCFRecord object
    let mut vcf_record = VCFRecord::new(vcf_reader.header().clone());
    let mut c: usize = 0;
    loop {
        let fetched = vcf_reader.next_record(&mut vcf_record)?;
        if !fetched {
            break;
        }
        c += 1;
    }
    Ok(c)
}

pub fn is_gzipped_stdin(lstdin: &mut Stdin) -> bool {
    let mut lock = lstdin.lock();
    let buf = lock.fill_buf().unwrap();
    match buf[0] {
        0x1f => match buf[1] {
            0x8b => {
                return true;
            }
            _ => {}
        },
        _ => {}
    }
    false
}

pub fn is_gzipped_file<T>(path: &T) -> Result<bool, std::io::Error>
where
    T: AsRef<Path>,
{
    let mut reader = BufReader::new(File::open(path)?);
    let mut itr = reader.fill_buf().into_iter().peekable();
    let values = itr.peek().unwrap();
    if values[0] == 0x1f && values[1] == 0x8b {
        Ok(true)
    } else {
        Ok(false)
    }
}
