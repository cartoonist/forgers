use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{stdout, BufWriter};
use std::path::PathBuf;
use vcf::{VCFError, VCFReader, VCFRecord, VCFWriter};

use crate::forge::{self, RegSiteMap};
use crate::vcf_util::{self, TBufReader, TBufWriter};

fn filter_vcf(
    mut vcf_writer: VCFWriter<TBufWriter>,
    mut vcf_reader: VCFReader<TBufReader>,
    ranks: RegSiteMap,
) -> Result<(), VCFError> {
    let mut vcf_record = VCFRecord::new(vcf_reader.header().clone());
    loop {
        let fetched = vcf_reader.next_record(&mut vcf_record)?;
        if fetched {
            let chrom = String::from_utf8(vcf_record.chromosome.to_vec())
                .expect("CHROM is not UTF-8 encoded");
            let pos = vcf_record.position;
            match ranks.get(&chrom) {
                Some(s) => {
                    if s.contains_key(&pos) {
                        vcf_writer.write_record(&vcf_record)?;
                    }
                }
                None => {}
            }
        } else {
            break;
        }
    }
    Ok(())
}

pub fn filter(
    output: PathBuf,
    input: PathBuf,
    forge_rank: PathBuf,
    top: f64,
    gzip: bool
) -> Result<(), VCFError> {
    let (vcf_reader, nr) = vcf_util::load_vcf_and_count(&input)?;
    let n = (top * nr as f64) as usize;
    let ranks = forge::load_rank(&forge_rank, n);
    match output {
        path if path == PathBuf::from("-") => {
            let writer: TBufWriter = BufWriter::new(if gzip {
                Box::new(GzEncoder::new(stdout(), Compression::default()))
            } else {
                Box::new(stdout())
            });
            let vcf_writer = VCFWriter::new(writer, &vcf_reader.header())?;
            filter_vcf(vcf_writer, vcf_reader, ranks)?;
            Ok(())
        }
        path => {
            let file = std::fs::File::create(&path).expect("Output file could not be created");
            let filename = path.display().to_string();
            let writer: TBufWriter = BufWriter::new(
                if gzip || filename.ends_with(".gz") || filename.ends_with(".bgz") {
                    Box::new(GzEncoder::new(file, Compression::default()))
                } else {
                    Box::new(file)
                },
            );
            let vcf_writer = VCFWriter::new(writer, &vcf_reader.header())?;
            filter_vcf(vcf_writer, vcf_reader, ranks)?;
            Ok(())
        }
    }
}
