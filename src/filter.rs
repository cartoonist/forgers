use std::path::PathBuf;
use std::io::{BufWriter, Write};
use flate2::write::GzEncoder;
use flate2::Compression;
use vcf::{VCFReader, VCFRecord, VCFWriter, VCFError};

use crate::forge::{self, RegSiteMap};
use crate::vcf_util::{self, TBufReader};

fn filter_vcf(
    mut vcf_reader: VCFReader<TBufReader>,
    ranks: RegSiteMap,
    output: BufWriter<Box<dyn Write>>,
) -> Result<(), VCFError> {
    let writer = GzEncoder::new(output, Compression::default());
    let mut vcf_writer = VCFWriter::new(writer, &vcf_reader.header())?;
    let mut vcf_record = VCFRecord::new(vcf_reader.header().clone());
    loop {
        let fetched = vcf_reader.next_record(&mut vcf_record)?;
        if fetched {
            let chrom = String::from_utf8(vcf_record.chromosome.to_vec()).unwrap();
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

pub fn filter(input: PathBuf, forge_rank: PathBuf, top: f64, output: PathBuf) {
    let (vcf_reader, nr) = vcf_util::load_vcf_and_count(&input).unwrap();
    let n = (top * nr as f64) as usize;
    let ranks = forge::load_rank(&forge_rank, n);
    match output {
        p if p == PathBuf::from("-") => {
            filter_vcf(
                vcf_reader,
                ranks,
                BufWriter::new(Box::new(std::io::stdout())),
            ).unwrap();
        }
        _ => {
            let file = std::fs::File::create(&output).unwrap();
            filter_vcf(vcf_reader, ranks, BufWriter::new(Box::new(file))).unwrap();
        }
    }
}
