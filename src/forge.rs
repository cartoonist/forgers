use log::warn;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

pub type SiteMap = HashMap<u64, usize>;
pub type RegSiteMap = HashMap<String, SiteMap>;

pub fn parse_id(id: &str) -> Option<(String, u64)> {
    let tokens: Vec<&str> = id.split(':').collect();
    if tokens.len() != 2 {
        return None;
    }
    match tokens[1].parse::<u64>() {
        Ok(pos) => Some((String::from(tokens[0]), pos)),
        Err(_) => None,
    }
}

pub fn load_rank<T>(path: T, top: f64) -> RegSiteMap
where
    T: AsRef<std::path::Path>,
{
    let mut file = File::open(path).expect("FORGe rank file not found");
    let reader = BufReader::new(&file);

    let nof_records = reader.lines().count();

    file.seek(SeekFrom::Start(0)).expect("Cannot seek to start of file");
    let reader = BufReader::new(&file);

    let n = (top * nof_records as f64) as usize;
    let mut smap = RegSiteMap::new();
    let mut i: usize = 1;
    for line in reader.lines() {
        let line = line.unwrap();
        let tokens = parse_id(&line);

        match tokens {
            Some((region, pos)) => {
                if smap.contains_key(&region) {
                    let rmap = smap.get(&region).unwrap();
                    if rmap.contains_key(&pos) {
                        warn!("Duplicated position: {}", &line);
                        continue;
                    }
                }
                smap.entry(region)
                    .or_insert(HashMap::new())
                    .entry(pos)
                    .or_insert(i);
                i += 1;
            }

            None => {
                warn!("Invalid input format: {}", &line);
            }
        }

        if i > n {
            break;
        }
    }

    smap
}
