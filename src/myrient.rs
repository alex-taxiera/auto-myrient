use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use colored::{Colorize, CustomColor};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use reqwest::blocking::Client;

use reqwest::header;
use retry::delay::Exponential;
use retry::retry;
use select::document::Document;
use select::predicate::{Attr, Name, Predicate};

use crate::constants;

const HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

const MAX_RETRIES: usize = 3;

const PROGRESS_PERCENT_PART: &str = "percent:>3";
const PROGESS_TEMPLATE: &str =
    " | {decimal_bytes:>9} / {decimal_total_bytes} | {bar} | ETA: {eta:>3} | {decimal_bytes_per_sec:>11}";

pub struct Catalog {
    pub title: String,
    pub url: String,
}
#[derive(Clone)]
pub struct Collection {
    pub title: String,
    pub url: String,
}
#[derive(Debug, Clone)]
pub struct Rom {
    pub name: String,
    pub file: String,
    pub url: String,
}
struct DownloadProgress<R> {
    inner: R,
    progress_bar: ProgressBar,
}

fn get_color_for_percentage(percent: f64) -> CustomColor {
    match percent {
        0.0..=0.1 => CustomColor::new(255, 0, 0),
        0.1..=0.2 => CustomColor::new(255, 51, 0),
        0.2..=0.3 => CustomColor::new(255, 102, 0),
        0.3..=0.4 => CustomColor::new(255, 153, 0),
        0.4..=0.5 => CustomColor::new(255, 204, 0),
        0.5..=0.6 => CustomColor::new(255, 255, 0),
        0.6..=0.7 => CustomColor::new(204, 255, 0),
        0.7..=0.8 => CustomColor::new(153, 255, 0),
        0.8..=0.9 => CustomColor::new(102, 255, 0),
        0.9..=0.99 => CustomColor::new(51, 255, 0),
        _ => CustomColor::new(0, 255, 0),
    }
}

fn build_progress_template(progress_bar: &ProgressBar, position: Option<f64>) -> String {
    let percent =
        position.unwrap_or(progress_bar.position() as f64) / progress_bar.length().unwrap() as f64;

    let color = get_color_for_percentage(percent);

    format!(
        "{}{}",
        format!("{{{}}}%", PROGRESS_PERCENT_PART).custom_color(color),
        format!("{}", PROGESS_TEMPLATE),
    )
}

impl<R: Read> Read for DownloadProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).map(|n| {
            self.progress_bar.set_style(
                ProgressStyle::with_template(
                    build_progress_template(&self.progress_bar, None).as_str(),
                )
                .unwrap(),
            );
            self.progress_bar.inc(n as u64);
            n
        })
    }
}

pub fn fetch(path: &String) -> Result<String, reqwest::Error> {
    let res = HTTP_CLIENT
        .get(format!("{}{}", constants::MYRIENT_HTTP_ADDR, path))
        .headers(constants::REQ_HEADERS.clone())
        .send();

    match res {
        Ok(res) => match res.text() {
            Ok(text) => {
                return Ok(text);
            }
            Err(e) => {
                return Err(e);
            }
        },
        Err(e) => {
            return Err(e);
        }
    }
}

pub fn get_collections(html: &String) -> Vec<Collection> {
    let dom = Document::from(html.as_str());
    let mut collections: Vec<Collection> = Vec::new();

    let main_dir = dom.find(Attr("id", "list").descendant(Name("tr")));

    for dir in main_dir {
        let cell_link = dir.find(Name("td").descendant(Name("a"))).next();

        match cell_link {
            None => continue,
            Some(cell_link) => match cell_link.attr("title") {
                None => continue,
                Some(title) => match cell_link.attr("href") {
                    None => continue,
                    Some(href) => {
                        collections.push(Collection {
                            title: title.to_string(),
                            url: href.to_string(),
                        });
                    }
                },
            },
        }
    }

    collections
}

pub fn get_catalog_url_by_name(html: &String, catalog_name: &str) -> Option<String> {
    let dom = Document::from(html.as_str());
    let main_dir = dom.find(Attr("id", "list").descendant(Name("tr")));
    for dir in main_dir {
        let cell_link = dir.find(Name("td").descendant(Name("a"))).next();

        match cell_link {
            None => continue,
            Some(cell_link) => match cell_link.attr("title") {
                None => continue,
                Some(title) => {
                    if title.contains(catalog_name) {
                        match cell_link.attr("href") {
                            None => continue,
                            Some(href) => {
                                return Some(String::from(href));
                            }
                        }
                    }
                }
            },
        }
    }

    None
}

pub fn get_catalogs(html: &String) -> Vec<Catalog> {
    let dom = Document::from(html.as_str());
    let mut catalogs: Vec<Catalog> = Vec::new();

    let main_dir = dom.find(Attr("id", "list").descendant(Name("tr")));

    for dir in main_dir {
        let cell_link = dir.find(Name("td").descendant(Name("a"))).next();

        match cell_link {
            None => continue,
            Some(cell_link) => match cell_link.attr("title") {
                None => continue,
                Some(title) => match cell_link.attr("href") {
                    None => continue,
                    Some(href) => {
                        catalogs.push(Catalog {
                            title: title.to_string(),
                            url: href.to_string(),
                        });
                    }
                },
            },
        }
    }

    catalogs
}

pub fn get_roms_for_collection(html: &String) -> HashMap<String, Rom> {
    let dom = Document::from(html.as_str());

    let mut roms: HashMap<String, Rom> = HashMap::new();

    let main_dir = dom.find(Attr("id", "list").descendant(Name("tr")));

    for rom in main_dir {
        let cell_link = rom.find(Name("a")).next();

        match cell_link {
            None => continue,
            Some(cell_link) => match cell_link.attr("title") {
                None => continue,
                Some(title) => match cell_link.attr("href") {
                    None => continue,
                    Some(href) => {
                        let name = Path::new(&title)
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        let rom = Rom {
                            name: name.to_string(),
                            file: title.to_string(),
                            url: href.to_string(),
                        };

                        roms.insert(name.to_string(), rom);
                    }
                },
            },
        }
    }

    roms
}

pub fn download_rom(
    output_path: &String,
    rom_url: &String,
    rom: &Rom,
    file_index: &usize,
    total_download_count: &usize,
) -> Result<File, reqwest::Error> {
    let local_path = Path::new(output_path).join(&rom.file);

    let mut resume_dl = false;
    let mut proceed_dl = true;

    let url = format!("{}{}", constants::MYRIENT_HTTP_ADDR, rom_url);
    let head = HTTP_CLIENT
        .head(url.clone())
        .headers(constants::REQ_HEADERS.clone())
        .send()?;

    let remote_file_size = head
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .unwrap_or(0);

    let local_file_size = if local_path.exists() {
        local_path.metadata().unwrap().len() - 1
    } else {
        0
    };
    if local_file_size == remote_file_size {
        proceed_dl = false
    } else if local_file_size > 0 {
        resume_dl = true
    }

    if proceed_dl {
        let mut request = HTTP_CLIENT
            .get(url.clone())
            .headers(constants::REQ_HEADERS.clone());

        let multi_progress = MultiProgress::new();

        let title_bar = multi_progress.add(ProgressBar::new(0));
        let progress_bar = multi_progress.add(ProgressBar::new(remote_file_size));

        title_bar.set_style(ProgressStyle::with_template("{prefix:.cyan}").unwrap());
        progress_bar.set_style(
            ProgressStyle::with_template(
                build_progress_template(
                    &progress_bar,
                    if resume_dl {
                        Some(local_file_size as f64)
                    } else {
                        None
                    },
                )
                .as_str(),
            )
            .unwrap()
            .progress_chars("=> "),
        );

        let width = total_download_count
            .checked_ilog10()
            .unwrap_or(0)
            .try_into()
            .unwrap_or(0)
            + 1;

        title_bar.set_prefix(format!(
            "{:11} {:width$}/{}: {}",
            if resume_dl { "Resuming" } else { "Downloading" },
            file_index,
            total_download_count,
            rom.name,
        ));

        if resume_dl {
            progress_bar.set_position(local_file_size);
            request = request.header(header::RANGE, &format!("bytes={}-", local_file_size));
        }

        let mut reader = DownloadProgress {
            inner: request.send()?,
            progress_bar,
        };

        let mut writer: std::fs::File = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&local_path)
            .unwrap();

        let _ = io::copy(&mut reader, &mut writer);

        reader.progress_bar.finish_and_clear();
        title_bar.finish_and_clear();

        println!(
            "{}",
            format!(
                "Downloaded  {}/{}: {}",
                file_index, total_download_count, rom.name
            )
            .green()
        );

        return Ok(writer);
    } else {
        println!(
            "{}",
            format!(
                "Already DLd {}/{}: {}",
                file_index, total_download_count, rom.name
            )
            .green()
        );
    }

    Ok(File::open(local_path).unwrap())
}

// custom error type that includes a vector of the failed Rom objects
#[derive(Debug, Clone)]
pub struct BulkDownloadError {
    pub failed_roms: Vec<Rom>,
}

impl fmt::Display for BulkDownloadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to download the following ROMs: ")?;
        for rom in &self.failed_roms {
            write!(f, "{} ", rom.name)?;
        }
        Ok(())
    }
}

pub fn download_roms(
    roms: &Vec<Rom>,
    output_dir: &String,
    catalog_url: &String,
    collection_url: &String,
) -> Result<(), BulkDownloadError> {
    let mut roms_with_errors: Vec<Rom> = Vec::new();

    for index in 0..roms.len() {
        let rom = roms.get(index).unwrap();
        let result = retry(Exponential::from_millis(100).take(MAX_RETRIES), || {
            download_rom(
                output_dir,
                &format!("{}{}{}", catalog_url, collection_url, rom.url),
                rom,
                &(index + 1),
                &roms.len(),
            )
        });

        if result.is_err() {
            println!("{}", format!("Error with  {}", rom.name).red());
            roms_with_errors.push(rom.clone());
        }
    }

    if roms_with_errors.len() > 0 {
        return Err(BulkDownloadError {
            failed_roms: roms_with_errors,
        });
    }

    Ok(())
}
