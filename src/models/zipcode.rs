use std::{collections::HashMap, fs::File, io::BufReader, sync::Arc};

use csv::ReaderBuilder;

pub type ZipCodeData = Arc<HashMap<String, (f64, f64)>>;

pub fn load_zipcode_data(path: &str) -> ZipCodeData {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(
            File::open(path).expect("Failed to open ZIP code file"),
        ));

    let mut map = HashMap::new();

    for result in reader.records() {
        if let Ok(record) = result {
            let zip = record.get(0).unwrap().to_string();
            let lat: f64 = record.get(1).unwrap().parse().unwrap_or(0.0);
            let lon: f64 = record.get(2).unwrap().parse().unwrap_or(0.0);
            map.insert(zip, (lat, lon));
        }
    }

    Arc::new(map)
}
