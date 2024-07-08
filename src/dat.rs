use std::path::Path;

use colored::Colorize;
use roxmltree::{Document, Error, ParsingOptions};

use crate::constants;

pub fn parse(dat_str: &str) -> Result<Document, Error> {
    let mut opt = ParsingOptions::default();
    opt.allow_dtd = true;
    return Document::parse_with_options(dat_str, opt);
}

pub fn get_wanted_roms(dat: &Document) -> Vec<String> {
    let mut wanted_roms: Vec<String> = Vec::new();

    let dat_root = dat.root_element();
    for child in dat_root.children() {
        if child.is_element() && child.tag_name().name() == "game" {
            for leaf in child.children() {
                if leaf.is_element() && leaf.tag_name().name() == "rom" {
                    match leaf.attribute("name") {
                        None => continue,
                        Some(name) => {
                            let filename = Path::new(&name)
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string();

                            if !wanted_roms.contains(&filename) {
                                wanted_roms.push(filename);
                            }
                        }
                    }
                }
            }
        }
    }

    wanted_roms
}

pub fn get_header_data(dat: &Document) -> (Option<String>, Option<String>) {
    let dat_root = dat.root_element();
    for child in dat_root.children() {
        if child.is_element() && child.tag_name().name() == "header" {
            let mut system = String::new();
            let mut catalog_name = None;

            for header_child in child.children() {
                // find system name
                if header_child.is_element() && header_child.tag_name().name() == "name" {
                    let text = header_child.text();
                    match text {
                        None => continue,
                        Some(text) => {
                            for fix in constants::DAT_POSTFIXES.iter() {
                                system = text.replace(fix, "");
                            }
                        }
                    }
                }

                // find catalog URL
                if header_child.is_element() && header_child.tag_name().name() == "url" {
                    match header_child.text() {
                        None => continue,
                        Some(text) => match constants::CATALOG_URLS.get(text) {
                            None => continue,
                            Some(catalog) => {
                                catalog_name = Some(catalog);
                            }
                        },
                    }
                }
            }

            match catalog_name {
                None => {
                    println!("{}", format!("Processing {}...", system.green()).green());

                    return (Some(system), None);
                }
                Some(catalog_name) => {
                    println!(
                        "{}",
                        format!("Processing {}: {}...", catalog_name.green(), system.green(),)
                            .green()
                    );

                    return (Some(system), Some(catalog_name.to_string()));
                }
            }
        }
    }

    (None, None)
}
