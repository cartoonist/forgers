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
            match ranks.get(&vcf_record.chromosome) {
                Some(s) => match s.get(&vcf_record.position) {
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
