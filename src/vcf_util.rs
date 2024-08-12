use flate2::read::MultiGzDecoder;
use std::fs::File;
use std::io::{BufReader, Read, BufWriter, Write};
use vcf::{VCFError, VCFReader, VCFRecord};

pub type TBufReader = BufReader<Box<dyn Read>>;
pub type TBufWriter = BufWriter<Box<dyn Write>>;


fn _load_vcf<T>(vcf_path: &T) -> Result<VCFReader<TBufReader>, VCFError>
where
    T: AsRef<std::path::Path>,
{
    let file = File::open(vcf_path).expect("Input VCF file not found");
    let filename = vcf_path.as_ref().display().to_string();
    let reader: TBufReader =
        BufReader::new(if filename.ends_with(".gz") || filename.ends_with(".bgz") {
            Box::new(MultiGzDecoder::new(file))
        } else {
            Box::new(file)
        });
    let vcf_reader = VCFReader::new(reader)?;
    Ok(vcf_reader)
}

pub fn nof_records<T>(vcf_path: &T) -> Result<usize, VCFError>
where
    T: AsRef<std::path::Path>,
{
    // prepare VCFRecord object
    let mut vcf_reader = _load_vcf(vcf_path)?;
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

pub fn load_vcf_and_count<T>(vcf_path: &T) -> Result<(VCFReader<TBufReader>, usize), VCFError>
where
    T: AsRef<std::path::Path>,
{
    let nr = nof_records(vcf_path)?;
    let vcf_reader = _load_vcf(vcf_path)?;
    Ok((vcf_reader, nr))
}

pub fn load_vcf<T>(vcf_path: &T) -> Result<VCFReader<TBufReader>, VCFError>
where
    T: AsRef<std::path::Path>,
{
    let vcf_reader = _load_vcf(vcf_path)?;
    Ok(vcf_reader)
}
