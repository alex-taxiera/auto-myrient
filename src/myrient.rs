use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;
use reqwest::blocking::Client;

use reqwest::header;
use select::document::Document;
use select::predicate::{Attr, Name, Predicate};

use crate::constants;

const HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());

pub struct Catalog {
    pub title: String,
    pub url: String,
}
#[derive(Clone)]
pub struct Collection {
    pub title: String,
    pub url: String,
}
#[derive(Clone)]
pub struct Rom {
    pub name: String,
    pub file: String,
    pub url: String,
}
struct DownloadProgress<R> {
    inner: R,
    progress_bar: ProgressBar,
}

impl<R: Read> Read for DownloadProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).map(|n| {
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

        let progress_bar = ProgressBar::new(remote_file_size);
        progress_bar.set_style(ProgressStyle::with_template(
            "{prefix:.cyan} {percent:>2}% | {bytes:>10} / {total_bytes} | {bar} | ETA: {eta:>3} | {bytes_per_sec:>12}",
        ).unwrap().progress_chars("=> "));

        
        let mut download_verb: &str = "Downloading";
        
        if resume_dl {
            download_verb = "Resuming";
            
            progress_bar.set_position(local_file_size);
            
            request = request.header(header::RANGE, &format!("bytes={}-", local_file_size));
        }
        
        progress_bar.set_prefix(format!(
            "{} {}/{}: {}",
            download_verb, file_index, total_download_count, rom.name
        ));

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
