use thiserror;


use reqwest::{header, IntoUrl};

use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use osrs_gph::config::{self, Config, Levels};

// Local dir
mod requirements;
mod parser;

use parser::*;

#[derive(Debug, thiserror::Error)]
enum Errors {
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTML Selector error")]
    Selector(#[from] scraper::error::SelectorErrorKind<'static>),
}

fn retrieve_webpage<S: IntoUrl>(url: S, overwrite: bool) -> Result<String, Errors> {
    const USER_AGENT: &str =
        "Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0";

    // Check if exists as a file already
    let path = PathBuf::from("src").join("bin").join("get_wiki_recipes").join("wiki_info").join("Money_making_guide.html");
    let read_from_file: bool = path.try_exists().is_ok_and(|x| x);

    let only_reading_old_file: bool = read_from_file && !overwrite;


    // Read from existing file
    if only_reading_old_file {
        let mut file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", path.display(), why),
            Ok(file) => file,
        };

        let mut body: String = String::new();
        match file.read_to_string(&mut body) {
            Err(why) => panic!("couldn't read from {}: {}", path.display(), why),
            Ok(_) => println!("successfully read from {}", path.display()),
        }

        return Ok(body)
    }


    // Request fresh API data instead
    dbg!("DOING API REQUEST");
    let client = reqwest::blocking::Client::new();
    let body: String = client.get(url)
        .header(header::USER_AGENT, USER_AGENT)
        .send()?
        .text()?;


    // Overwrite file with the new body data
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
        Ok(file) => file,
    };
    file.set_len(0)?;
    match file.write(body.as_bytes()) {
        Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
        Ok(_) => println!("successfully wrote to {}", path.display()),
    }
    file.sync_all().expect("Failed to sync data to filesystem");

    Ok(body)
}

fn main() -> Result<(), Errors> {
    let config: Config = config::load_config("config.yaml");
    let config_levels: Levels = config.levels;

    let url: &str = "https://oldschool.runescape.wiki/w/Money_making_guide";
    let body: String = retrieve_webpage(url, false)?;
    let html = Html::parse_document(&body);

    let table = extract_table(&html, 1).expect("Table was empty");

    //dbg!(&table);
    let rows = (1..=table.len())
        .map(|row_num| 
            extract_row(&table, row_num).expect("Row {row_num} is empty")
        )
        .collect::<Vec<_>>();

    // TODO: Name
    let possible_methods_idx: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)|
            has_required_levels_for_method(&config_levels, row)
                .then_some(idx)
        )
        .collect();

    //dbg!(&possible_methods_idx);

    let possible_methods_rows: Vec<&TableRow> = possible_methods_idx
        .iter()
        .map(|&i| {
            rows.get(i).expect("Row missing at index {i}")
        })
        .collect();

    //dbg!(&possible_methods_rows);
    let possible_methods_names: Vec<String> = possible_methods_rows
        .iter()
        .map(|row| {
            extract_column(row, 1).0
                .first()
                .expect("Missing method name")
                .value()
                .attr("title")
                .expect("Missing title")
                .replace("Money making guide/", "")
        })
        .collect();

    // Write results to a file
    // Overwrite file with new body data
    let path = PathBuf::from("src").join("bin").join("get_wiki_recipes").join("wiki_info").join("wiki_allowed_recipes.txt");
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
        Ok(file) => file,
    };
    file.set_len(0)?;

    for method in possible_methods_names {
        if let Err(e) = writeln!(file, "{method}") {
            panic!("Failed to write line: {e}");
        }
    }

    println!("Successfully wrote to {}", path.display());

    file.sync_all().expect("Failed to sync data to filesystem");

    Ok(())
}
