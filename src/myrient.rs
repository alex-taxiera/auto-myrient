use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;

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
impl Rom {
    pub fn to_string(&self) -> String {
        format!("{}: {}", self.name, self.file)
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

// def download(outputPath: str, wantedfile: RomMeta, fileIndex: int, totalDownloadCount: int):
//     resumedl = False
//     proceeddl = True

//     if platform.system() == 'Linux':
//         localpath = f'{outputPath}/{wantedfile["file"]}'
//     elif platform.system() == 'Windows':
//         localpath = f'{outputPath}\{wantedfile["file"]}'

//     logger(f'Checking    {str(fileIndex).zfill(len(str(totalDownloadCount)))}/{totalDownloadCount}: {wantedfile["name"]}', 'cyan')
//     resp = start_stream(wantedfile)

//     remotefilesize = int(resp.headers.get('content-length'))

//     if os.path.isfile(localpath):
//         localfilesize = int(os.path.getsize(localpath))
//         if localfilesize != remotefilesize:
//             resumedl = True
//         else:
//             proceeddl = False

//     if proceeddl:
//         file = open(localpath, 'ab')

//         size, unit = scale1024(remotefilesize)
//         pbar = ProgressBar(widgets=['\033[96m', Percentage(), ' | ', DataSize(), f' / {round(size, 1)} {unit}', ' ', Bar(marker='#'), ' ', ETA(), ' | ', FileTransferSpeed(), '\033[00m'], max_value=remotefilesize, redirect_stdout=True, max_error=False)
//         pbar.start()

//         if resumedl:
//             logger(f'Resuming    {str(fileIndex).zfill(len(str(totalDownloadCount)))}/{totalDownloadCount}: {wantedfile["name"]}', 'cyan', rewrite=True)
//             pbar += localfilesize
//             headers = {'Range': f'bytes={localfilesize}-'}
//             resp = start_stream(wantedfile, headers)
//             for data in resp.iter_content(chunk_size=CHUNKSIZE):
//                 file.write(data)
//                 pbar += len(data)
//         else:
//             logger(f'Downloading {str(fileIndex).zfill(len(str(totalDownloadCount)))}/{totalDownloadCount}: {wantedfile["name"]}', 'cyan', rewrite=True)
//             for data in resp.iter_content(chunk_size=CHUNKSIZE):
//                 file.write(data)
//                 pbar += len(data)

//         file.close()
//         pbar.finish()
//         print('\033[1A', end='\x1b[2K')
//         logger(f'Downloaded  {str(fileIndex).zfill(len(str(totalDownloadCount)))}/{totalDownloadCount}: {wantedfile["name"]}', 'green', True)
//     else:
//         logger(f'Already DLd {str(fileIndex).zfill(len(str(totalDownloadCount)))}/{totalDownloadCount}: {wantedfile["name"]}', 'green', True)

pub fn download_rom(
    output_path: &String,
    rom_url: &String,
    rom: &Rom,
    file_index: &usize,
    total_download_count: &usize,
) {
    let local_path = Path::new(output_path).join(&rom.file);

    let mut resume_dl = false;
    let mut proceed_dl = true;

    println!(
        "Checking    {}/{}: {}",
        file_index, total_download_count, rom.name
    );

    let url = format!("{}{}", constants::MYRIENT_HTTP_ADDR, rom_url);
    let mut headers = constants::REQ_HEADERS.clone();
    let resp = HTTP_CLIENT
        .get(url.clone())
        .headers(headers.clone())
        .send()
        .unwrap();

    let remote_file_size = resp.content_length().unwrap();

    let local_file_size = if local_path.exists() {
        Path::new(&local_path).metadata().unwrap().len()
    } else {
        0
    };
    if local_file_size == remote_file_size {
        proceed_dl = false
    } else if local_file_size > 0 {
        resume_dl = true
    }

    if proceed_dl {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&local_path)
            .unwrap();

        let mut writer = io::BufWriter::new(file);

        let size = remote_file_size as f32 / 1024.0;
        let unit = "KB";
        let pbar = ProgressBar::new(remote_file_size as u64);

        let style = ProgressStyle::default_bar()
            .template("{prefix:.cyan} {wide_bar} {pos}/{len} {per_sec} {eta}");

        if style.is_err() {
            println!("{}", style.err().unwrap());
        } else {
            pbar.set_style(style.unwrap().progress_chars("=> "));
        }
        pbar.set_prefix(format!(
            "Downloading {}/{}: {} / {:.1} {}",
            file_index, total_download_count, rom.name, size, unit
        ));

        if resume_dl {
            println!(
                "Resuming    {}/{}: {}",
                file_index, total_download_count, rom.name
            );
        } else {
            println!(
                "Downloading {}/{}: {}",
                file_index, total_download_count, rom.name
            );
        }

        let mut start = local_file_size;
        pbar.set_position(start);
        while start < remote_file_size {
            let end = (start + constants::CHUNK_SIZE).min(remote_file_size);
            headers.insert(
                header::RANGE,
                header::HeaderValue::from_str(&format!("bytes={}-{}", start, end - 1)).unwrap(),
            );

            let resp = HTTP_CLIENT
                .get(url.clone())
                .headers(headers.clone())
                .send()
                .unwrap();

            let data = resp.bytes().unwrap();

            writer.write_all(data.as_ref()).unwrap();
            pbar.inc(data.len() as u64);

            start = end;
        }
    } else {
        println!(
            "Already DLd {}/{}: {}",
            file_index, total_download_count, rom.name
        );
    }
}
