use anyhow::Context;
use clap::Parser;
use regex::Regex;
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
	#[arg(short, long)]
	input_file: String,

	#[arg(short, long)]
	tracker_file: String,

	#[arg(short, long)]
	metadata_dir: String,

	#[arg(short, long)]
	units_data_dir: String,
}

#[derive(Debug, Serialize)]
struct BuildMetadata<'a> {
	#[serde(rename = "t")]
	total_time: f64,

	#[serde(rename = "r")]
	rustc_version: &'a str,

	#[serde(rename = "u")]
	total_units: usize,

	/// Build start timestamp in seconds
	#[serde(rename = "b")]
	build_start_unix_timestamp: u64,
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

fn ensure_directory_exists(path: &Path) -> anyhow::Result<()> {
	fs::create_dir_all(path).context("Failed to create directory")
}

fn ensure_tracker_file_exists(path: &Path) -> anyhow::Result<()> {
	if !path.exists() {
		write_json_file(path, &Vec::<String>::new())?;
	}
	Ok(())
}

fn write_json_file<T: Serialize>(path: &Path, data: &T) -> anyhow::Result<()> {
	let json = serde_json::to_string(data)?;
	fs::write(path, json).context("Failed to write JSON file")
}

fn main() -> anyhow::Result<()> {
	let args = CliArgs::parse();
	let input_path = Path::new(&args.input_file);
	let tracker_path = Path::new(&args.tracker_file);
	let metadata_dir = Path::new(&args.metadata_dir);
	let units_dir = Path::new(&args.units_data_dir);

	ensure_directory_exists(metadata_dir)?;
	ensure_directory_exists(units_dir)?;
	ensure_tracker_file_exists(tracker_path)?;

	let input_filename = input_path.file_name().context("Invalid input filename")?.to_str().unwrap();
	let html_content = fs::read_to_string(input_path).context("Failed to read input file")?;

	let mut tracker: BuildTracker = serde_json::from_str(&fs::read_to_string(tracker_path)?)?;
	if tracker.contains(&input_filename.to_string()) {
		anyhow::bail!("File with this timestamp has already been processed");
	}

	let build_metadata = BuildMetadata {
		total_time: extract_value(&html_content, r"<td>Total time:</td><td>(\d+(?:\.\d+)?)s").unwrap().parse()?,
		rustc_version: extract_value(&html_content, r"<td>rustc:</td><td>(rustc [\d\.\w-]+)").unwrap(),
		total_units: extract_value(&html_content, r"<td>Total units:</td><td>(\d+)").unwrap().parse()?,
		build_start_unix_timestamp: extract_value(input_filename, r"(\d{8}T\d{6}Z)").unwrap().parse()?, // TODO: this parse doesn't work, use chrono to parse
	};

	let units_data = extract_units_data(&html_content);

	write_json_file(&metadata_dir.join(format!("{}.json", build_metadata.build_start_unix_timestamp)), &build_metadata)?;

	write_json_file(&units_dir.join(format!("{}.json", build_metadata.build_start_unix_timestamp)), &units_data)?;

	tracker.push(input_filename.to_string());
	write_json_file(tracker_path, &tracker)?;

	Ok(())
}
