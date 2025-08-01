use anyhow::Context;
use chrono::NaiveDateTime;
use clap::Parser;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
	/// Directory with raw HTML files generated from `cargo build --timings`
	/// (with their original filenames i.e not tampered after generating from `cargo build --timings`)
	#[arg(short)]
	raw_html_files_dir: PathBuf,
	/// JSON file to store processed HTML files timestamps
	#[arg(short)]
	tracker_file: PathBuf,
	/// JSON file to store metadata of all processed HTML files
	#[arg(short)]
	metadatas_file: PathBuf,
	/// Directory to store build units JSON files(1 per HTML file)
	#[arg(short)]
	units_data_dir: PathBuf,
}

type BuildMetadatas = HashMap<String, BuildMetadata>;

#[derive(Debug, Serialize, Deserialize)]
struct BuildMetadata {
	#[serde(rename = "t")]
	total_time: f64,
	#[serde(rename = "r")]
	rustc_version: String,
	#[serde(rename = "u")]
	total_units: usize,
	/// Build start timestamp in seconds
	#[serde(rename = "b")]
	build_start_unix_timestamp: u64,
	/// Commit hash in `debate-map/app`, which triggered cargo timings
	#[serde(rename = "h")]
	commit_hash: String,
}

#[derive(Debug, Serialize)]
struct UnitBuildData {
	#[serde(rename = "u")]
	name: String,

	#[serde(rename = "t")]
	time: f64,
}

type BuildTracker = Vec<String>;
type UnitsBuildData = Vec<UnitBuildData>;

fn extract_units_data(content: &str) -> UnitsBuildData {
	let table_re = Regex::new(r#"(?s)<table class="my-table">.*?<tbody>\s*(.*?)</tbody>"#).unwrap();
	let row_re = Regex::new(r#"<tr>\s*<td>\d+\.</td>\s*<td>(.*?)</td>\s*<td>(\d+(?:\.\d+)?s)</td>"#).unwrap();

	table_re.captures(content).and_then(|cap| cap.get(1)).map(|table| row_re.captures_iter(table.as_str()).map(|row| UnitBuildData { name: row[1].to_string(), time: row[2].trim_end_matches('s').parse().unwrap() }).collect()).unwrap_or_default()
}
fn extract_value<'a>(content: &'a str, pattern: &str) -> Option<&'a str> {
	Regex::new(pattern).ok()?.captures(content).and_then(|cap| cap.get(1)).map(|m| m.as_str())
}

fn write_json_file<P: AsRef<Path>, T: Serialize>(path: P, data: &T) -> anyhow::Result<()> {
	let json = serde_json::to_string(data)?;
	fs::write(path, json).context("Failed to write JSON file")
}

fn extract_raw_time_and_commit_hash<T: AsRef<str>>(filename: T) -> anyhow::Result<(String, String)> {
	let mut raw_time = extract_value(filename.as_ref(), r"(\d{8}T\d{6})").context("Failed to extract raw time")?.to_string();
    raw_time.push('Z');
	let commit_hash = filename.as_ref().rsplit('_').next().unwrap().trim_end_matches(".html").to_string();
	Ok((raw_time, commit_hash))
}

fn main() -> anyhow::Result<()> {
	let args = CliArgs::parse();

	if !args.raw_html_files_dir.exists() {
		anyhow::bail!("Directory with raw HTML files does not exist");
	}

	if !args.tracker_file.exists() {
		write_json_file(&args.tracker_file, &Vec::<String>::new())?;
	}

	if !args.metadatas_file.exists() {
		anyhow::bail!("Metadatas file does not exist");
	}

	if !args.units_data_dir.exists() {
		anyhow::bail!("Units data directory does not exist");
	}

    let raw_html_files = fs::read_dir(&args.raw_html_files_dir)?.map(|entry| entry.map(|e| e.path())).collect::<Result<Vec<_>, _>>()?;
    if raw_html_files.is_empty() {
        anyhow::bail!("No raw html files found in the directory");
    }

    let mut tracker: BuildTracker = serde_json::from_str(&fs::read_to_string(&args.tracker_file)?)?;
    let mut metadatas: BuildMetadatas = serde_json::from_str(&fs::read_to_string(&args.metadatas_file)?)?;

    for raw_html_file in raw_html_files {
        let input_filename = raw_html_file.file_name().context("Invalid input filename")?.to_str().unwrap();
        println!("Processing file: {}", input_filename);

        let (raw_time, commit_hash) = extract_raw_time_and_commit_hash(input_filename)?;
        println!("Found raw time: {}, commit hash: {}", raw_time, commit_hash);

        if !tracker.contains(&raw_time.to_string()) {
            println!("[Extracting] New raw time found: {}", raw_time);
            let build_start_unix_timestamp = {
                let parsed = NaiveDateTime::parse_from_str(&raw_time, "%Y%m%dT%H%M%SZ")?.and_utc();
                parsed.timestamp() as u64
            };

            let html_content = fs::read_to_string(&raw_html_file).context("Failed to read input file")?;
            let build_metadata = BuildMetadata {
                total_time: extract_value(&html_content, r"<td>Total time:</td><td>(\d+(?:\.\d+)?)s").unwrap().parse()?,
                rustc_version: extract_value(&html_content, r"<td>rustc:</td><td>(rustc [\d\.\w-]+)").unwrap().to_string(),
                total_units: extract_value(&html_content, r"<td>Total units:</td><td>(\d+)").unwrap().parse()?,
                build_start_unix_timestamp,
                commit_hash,
            };

            let units_data = extract_units_data(&html_content);

            write_json_file(&args.units_data_dir.join(format!("units_{raw_time}.json")), &units_data)?;
            metadatas.insert(raw_time.to_string(), build_metadata);
            tracker.push(raw_time.to_string());
        }
        println!("{}", "-".repeat(50));
    }

    write_json_file(&args.tracker_file, &tracker)?;
    write_json_file(&args.metadatas_file, &metadatas)?;

	Ok(())
}

#[cfg(test)]
mod tests {
	use chrono::{Datelike, Timelike};
    use chrono::{NaiveDateTime, TimeZone, Utc};

	#[test]
	fn test_raw_time_with_nanos_and_commit_hash_extraction_and_parsing() -> anyhow::Result<()> {
        // in new rust verions, the filename format is changed to include nanoseconds
		let filename = "cargo-timing-20250725T214437.067643888Z_300007d.html";
		let (raw_time, commit_hash) = super::extract_raw_time_and_commit_hash(filename)?;
		assert_eq!(raw_time, "20250725T214437Z");
		assert_eq!(commit_hash, "300007d");

		let parsed_time = NaiveDateTime::parse_from_str(&raw_time, "%Y%m%dT%H%M%SZ")?.and_utc();
		assert_eq!(parsed_time.year(), 2025);
		assert_eq!(parsed_time.month(), 7);
		assert_eq!(parsed_time.day(), 25);
		assert_eq!(parsed_time.hour(), 21);
		assert_eq!(parsed_time.minute(), 44);
		assert_eq!(parsed_time.second(), 37);

		Ok(())
	}

    #[test]
    fn test_raw_time_without_nanos_and_commit_hash_extraction_and_parsing() -> anyhow::Result<()> {
        // in old rust verions, the filename format is changed to include nanoseconds
        let filename = "cargo-timing-20241101T173901Z_73191b7.html";
        let (raw_time, commit_hash) = super::extract_raw_time_and_commit_hash(filename)?;

        assert_eq!(raw_time, "20241101T173901Z");
        assert_eq!(commit_hash, "73191b7");

        let parsed_time = NaiveDateTime::parse_from_str(&raw_time, "%Y%m%dT%H%M%SZ")?;
        let utc_time = Utc.from_utc_datetime(&parsed_time);

        assert_eq!(utc_time.year(), 2024);
        assert_eq!(utc_time.month(), 11);
        assert_eq!(utc_time.day(), 1);
        assert_eq!(utc_time.hour(), 17);
        assert_eq!(utc_time.minute(), 39);
        assert_eq!(utc_time.second(), 1);

        Ok(())
    }
}
