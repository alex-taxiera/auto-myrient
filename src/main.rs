use clap::Parser;
use colored::Colorize;
use std::{
    fmt::Debug,
    io::{self, Write},
    path::Path,
};

mod constants;
mod dat;
mod myrient;

/// Tool for bulk downloading from Myrient
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input DAT-file containing wanted ROMs
    #[arg(short, long)]
    input: String,

    /// Output path for ROM files to be downloaded
    #[arg(short, long)]
    output: String,

    /// Choose catalog manually, even if automatically found
    #[arg(short, long)]
    catalog: bool,

    /// Choose system collection manually, even if automatically found
    #[arg(short, long)]
    system: bool,

    /// List only ROMs that are not found in server (if any)
    #[arg(short, long)]
    list: bool,
}

fn main() {
    let args = Args::parse();

    validate_args(&args);

    let mut output_dir = args.output;
    if cfg!(windows) && output_dir.ends_with("\\") {
        output_dir = output_dir[..output_dir.len() - 1].to_string();
    } else if cfg!(unix) && output_dir.ends_with("/") {
        output_dir = output_dir[..output_dir.len() - 1].to_string();
    }

    println!("{}", format!("Output directory: {}", output_dir).green());

    println!("{}", "Opening input DAT-file...".green());
    let dat_file_res = std::fs::read_to_string(&args.input);
    if dat_file_res.is_err() {
        println!("{}", "Error opening DAT-file!".red());
        std::process::exit(1);
    }

    let dat_file: String = dat_file_res.unwrap();
    let dat_res = dat::parse(&dat_file);
    if dat_res.is_err() {
        println!("{}", "Error parsing DAT-file!".red());
        std::process::exit(1);
    }

    let dat = dat_res.unwrap();

    let (system_res, catalog_name_res) = dat::get_header_data(&dat);
    let system = system_res.unwrap_or(String::new());
    let catalog_name = catalog_name_res.unwrap_or(String::new());

    let catalog_url = get_catalog_url(&catalog_name, &args.catalog);

    let collection_url = get_collection_url(&catalog_url, &system, &args.system);

    let collection_res = myrient::fetch(&format!("{}{}", &catalog_url, &collection_url));
    let collection_html = collection_res.unwrap_or_else(|e| {
        println!("{}", e);
        std::process::exit(1);
    });

    let wanted_rom_names = dat::get_wanted_roms(&dat);
    let available_roms = myrient::get_roms_for_collection(&collection_html);

    let mut missing_roms: Vec<String> = Vec::new();
    let mut wanted_roms: Vec<myrient::Rom> = Vec::new();

    for wanted_rom_name in wanted_rom_names.iter() {
        if available_roms.contains_key(wanted_rom_name) {
            wanted_roms.push(available_roms.get(wanted_rom_name).unwrap().clone());
        } else {
            missing_roms.push(wanted_rom_name.to_string());
        }
    }

    let missing_roms_len = missing_roms.len();

    println!(
        "{}",
        format!(
            "Amount of wanted ROMs in DAT-file   : {}",
            wanted_roms.len().to_string()
        )
        .green()
    );
    println!(
        "{}",
        format!(
            "Amount of found ROMs at server      : {}",
            available_roms.len().to_string()
        )
        .green()
    );
    if missing_roms_len > 0 {
        println!(
            "{}",
            format!(
                "Amount of missing ROMs at server    : {}",
                missing_roms_len.to_string()
            )
            .yellow()
        );
    }

    if !args.list {
        for index in 0..wanted_roms.len() {
            let wanted_rom = wanted_roms.get(index).unwrap();
            let file = myrient::download_rom(
                &output_dir,
                &format!("{}{}{}", &catalog_url, &collection_url, wanted_rom.url),
                &wanted_rom,
                &(index + 1),
                &wanted_roms.len(),
            );

            if file.is_err() {
                println!("{}", file.unwrap_err().to_string().red());
            }
        }

        println!("{}", "Downloading complete!".green());
    }

    if missing_roms_len > 0 {
        println!(
            "{}",
            format!(
                "Following {} ROMs in DAT not automatically found from server, grab these manually:",
                missing_roms_len
            )
            .red()
        );

        for missing_rom in missing_roms.iter() {
            println!("{}", missing_rom.yellow());
        }
    } else {
        println!("{}", "All wanted ROMs found from server!".green());
    }
}

fn validate_args(args: &Args) {
    if !Path::new(&args.input).is_file() {
        println!("{}", "Invalid input DAT-file!".red());
        std::process::exit(1);
    }

    if !Path::new(&args.output).is_dir() {
        println!("{}", "Invalid output ROM path!".red());
        std::process::exit(1);
    }
}

fn get_catalog_url(catalog_name: &String, select_catalog: &bool) -> String {
    let res = myrient::fetch(&String::new());

    let html = res.unwrap_or_else(|e| {
        println!("{}", e);
        std::process::exit(1);
    });

    let mut catalog_url: Option<String> = None;

    if !catalog_name.is_empty() {
        let url_res = myrient::get_catalog_url_by_name(&html, catalog_name);
        if url_res.is_some() {
            catalog_url = Some(url_res.unwrap().to_string());
        }
    }

    if catalog_url.is_none() || *select_catalog {
        // logger('Catalog for DAT not automatically found, please select from the following:', 'yellow')
        println!(
            "{}",
            "Catalog for DAT not automatically found, please select from the following:".yellow()
        );
        let catalogs = myrient::get_catalogs(&html);

        for index in 0..catalogs.len() {
            println!(
                "{}",
                format!(
                    "{}: {}",
                    (index + 1).to_string(),
                    catalogs.get(index).unwrap().title
                )
                .cyan()
            );
        }

        loop {
            print!("{}", "Input selected catalog number: ".cyan());
            io::stdout().flush().unwrap();
            let mut catalog_choice = String::new();
            let _ = io::stdin().read_line(&mut catalog_choice);
            let num_test = catalog_choice.trim().parse::<usize>();

            match num_test {
                Ok(num) => {
                    if num > 0 && num <= catalogs.len() {
                        return catalogs.get(&num - 1).unwrap().url.to_string();
                    } else {
                        println!("{}", "Input number out of range!".red());
                    }
                }
                Err(_) => {
                    println!("{}", "Invalid number!".red());
                }
            }
        }
    }

    return catalog_url.unwrap();
}

fn get_collection_url(catalog_url: &String, system_name: &String, select_system: &bool) -> String {
    let res = myrient::fetch(&catalog_url);
    let html = res.unwrap_or_else(|e| {
        println!("{}", e);
        std::process::exit(1);
    });

    let collections = myrient::get_collections(&html);
    let collections_len = collections.len();
    let mut matching_collections: Vec<myrient::Collection> = Vec::new();

    let mut collection_url: Option<String> = None;

    if !system_name.is_empty() {
        for collection in collections.clone() {
            if collection.title.contains(system_name) {
                matching_collections.push(collection)
            }
        }

        if matching_collections.len() == 1 {
            collection_url = Some(matching_collections.get(0).unwrap().url.to_string());
        }
    }

    if collection_url.is_none() || *select_system {
        println!(
            "{}",
            "Collection for DAT not automatically found, please select from the following:"
                .yellow()
        );

        let use_matches = matching_collections.len() > 1 && !*select_system;

        if use_matches {
            for index in 0..matching_collections.len() {
                println!(
                    "{}",
                    format!(
                        "{}: {}",
                        (index + 1).to_string(),
                        matching_collections.get(index).unwrap().title
                    )
                    .yellow()
                );
            }
        } else {
            for index in 0..collections_len {
                println!(
                    "{}",
                    format!(
                        "{}: {}",
                        (index + 1).to_string(),
                        collections.get(index).unwrap().title
                    )
                    .cyan()
                );
            }
        }

        loop {
            print!("{}", "Input selected collection number: ".cyan());
            io::stdout().flush().unwrap();
            let mut collection_choice = String::new();
            let _ = io::stdin().read_line(&mut collection_choice);
            let num_test = collection_choice.trim().parse::<usize>();

            match num_test {
                Ok(num) => {
                    if use_matches {
                        if num > 0 && num <= matching_collections.len() {
                            return matching_collections.get(&num - 1).unwrap().url.to_string();
                        } else {
                            println!("{}", "Input number out of range!".red());
                        }
                    } else if num > 0 && num <= collections_len {
                        return collections.get(&num - 1).unwrap().url.to_string();
                    } else {
                        println!("{}", "Input number out of range!".red());
                    }
                }
                Err(_) => {
                    println!("{}", "Invalid number!".red());
                }
            }
        }
    }

    collection_url.unwrap()
}
