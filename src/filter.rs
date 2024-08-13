use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use vcf::{VCFError, VCFReader, VCFRecord, VCFWriter};

use crate::forge;

pub fn filter<T, W, R>(
    mut vcf_writer: VCFWriter<BufWriter<W>>,
    mut vcf_reader: VCFReader<BufReader<R>>,
    forge_rank: &T,
    top: f64,
    annotate: bool,
    info_key: &String,
) -> Result<(), VCFError>
where
    T: AsRef<Path>,
    W: Write,
    R: Read,
{
    let ranks = forge::load_rank(forge_rank, top);
    let mut vcf_record = VCFRecord::new(vcf_reader.header().clone());
    loop {
        let fetched = vcf_reader.next_record(&mut vcf_record)?;
        if fetched {
            let chrom = String::from_utf8(vcf_record.chromosome.to_vec())
                .expect("CHROM is not UTF-8 encoded");
            let pos = vcf_record.position;
            match ranks.get(&chrom) {
                Some(s) => match s.get(&pos) {
                    Some(fr) => {
                        if annotate {
                            vcf_record.insert_info(
                                info_key.as_bytes(),
                                vec![format!("{}", fr).as_bytes().to_vec()],
                            );
                        }
                        vcf_writer.write_record(&vcf_record)?;
                    }
                    None => {}
                },
                None => {}
            }
        } else {
            break;
        }
    }
    Ok(())
}
