use std::{fs, path::{Path, PathBuf}};
use csv::WriterBuilder;
use crate::data::{CandleSticks, DataManager};

lazy_static::lazy_static! {
    static ref INSTANCE_ID: String =  std::process::id().to_string();
}


impl DataManager {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path)
            .expect("Failed to create data directory..");

        Self { base_path }
    }

    pub fn get_csv_path(&self, symbol: &str, timeframe: i64)
    -> PathBuf
    {
        let symbol_dir = self.base_path.join(symbol.to_uppercase());
        fs::create_dir_all(&symbol_dir).unwrap_or_default();
        symbol_dir.join(format!("{}_{}_{}.csv", timeframe,
            chrono::Utc::now().format("%y%m%d"), *INSTANCE_ID)
        )

    }

    pub fn save_to_csv(&self,
        symbol: &str,
        timeframe: i64,
        candles: &[CandleSticks]
    ) -> Result<PathBuf, anyhow::Error> 
    {
        if candles.is_empty() {
            return Err(anyhow::anyhow!("No data to save.."));
        }

        let file_path = self.get_csv_path(symbol, timeframe);

        let mut writer = WriterBuilder::new()
            .has_headers(true)
            .from_path(&file_path)?;

        for data in candles {
            writer.serialize(data)?;
        }

        writer.flush()?;
        println!("Saved {} to {}", candles.len(), file_path.display());
        Ok(file_path)
    }
}