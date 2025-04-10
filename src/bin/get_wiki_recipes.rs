use std::fmt::Display;
use std::io::{Read, Write};
use std::iter::zip;
use std::path::Path;
use std::fs::File;
use reqwest::{IntoUrl, header};
use scraper::{node::Element, ElementRef, Html, Selector};

use std::ops::Not;
use osrs_gph::config::{self, Config, Levels};

#[derive(Debug)]
enum HTMLError<'a> {
    SelectorError(scraper::error::SelectorErrorKind<'a>),
}

#[derive(Debug)]
enum Errors<'a> {
    RequestError(reqwest::Error),
    IoError(std::io::Error),
    HTMLError(HTMLError<'a>),
}

impl<'a> From<reqwest::Error> for Errors<'a> {
    fn from(value: reqwest::Error) -> Self {
        Errors::RequestError(value)
    } 
}

impl<'a> From<std::io::Error> for Errors<'a> {
    fn from(value: std::io::Error) -> Self {
        Errors::IoError(value)
    } 
}

impl<'a> From<HTMLError<'a>> for Errors<'a> {
    fn from(value: HTMLError<'a>) -> Self {
        Errors::HTMLError(value)
    } 
}

impl<'a> From<scraper::error::SelectorErrorKind<'a>> for HTMLError<'a> {
    fn from(value: scraper::error::SelectorErrorKind<'a>) -> Self {
        HTMLError::SelectorError(value)
    }
}


fn retrieve_webpage<'a, S: IntoUrl>(url: S, overwrite: bool) -> Result<String, Errors<'a>> {
    // Check if exists as a file already
    let path: &Path = Path::new("src\\bin\\wiki_info\\Money_making_guide.html");
    let read_from_file: bool = path.try_exists().is_ok_and(|x| x == true);
    let body = if read_from_file {
        // Read from existing file
        let mut file = match File::open(&path) {
            Err(why) => panic!("couldn't open {}: {}", path.display(), why),
            Ok(file) => file,
        };

        let mut body: String = String::new();
        match file.read_to_string(&mut body) {
            Err(why) => panic!("couldn't read from {}: {}", path.display(), why),
            Ok(_) => println!("successfully read from {}", path.display()),
        }

        body
    } else {
        // Just return api data
        dbg!("DOING API REQUEST");
        let client = reqwest::blocking::Client::new();
        let user_agent: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X x.y; rv:42.0) Gecko/20100101 Firefox/42.0";
        client.get(url)
            .header(header::USER_AGENT, user_agent)
            .send()?.text()?
    };

    if overwrite || (!read_from_file) {
        // Overwrite file with new body data
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
    };


    Ok(body)

}

fn make_selector<'a: 'b, 'b>(selectors: &'a str) -> Result<Selector, HTMLError<'b>>  {
    Selector::parse(selectors).map_err(HTMLError::<'b>::from)
}

fn extract_table<'a, 'b: 'a>(html: &'b Html, table_number: usize) -> Option<Vec<Vec<ElementRef<'a>>>> {
    assert!(table_number >= 1);

    let table_selector: Selector = make_selector(".wikitable")
        .expect("Failed to make table selector");
    let table_header = html.select(&table_selector).clone()
        .take(table_number)
        .nth(table_number - 1)
        .expect(&format!("Failed to find table {table_number} in HTML"));

    let contents: Vec<Vec<ElementRef<'a>>> = table_header.child_elements()
        .nth(1) // Skip caption
        .expect("Failed to find table body from header")
        .child_elements()
        .skip(1)
        .filter_map(|elementref| { // Filter to remove any empty rows
            let x = elementref.child_elements()
            .collect::<Vec<ElementRef<'a>>>();
            x.is_empty().not().then(|| x)
        }).collect::<Vec<_>>();

    // If all rows are empty return Ok(None)
    contents.is_empty().not().then(|| contents)
}

fn extract_row<'a, 'b>(table: &'b Vec<Vec<ElementRef<'a>>>, row_number: usize) -> Option<Vec<Vec<ElementRef<'a>>>>{
    assert!(row_number >= 1);

    let row: Vec<Vec<ElementRef<'a>>> = table.into_iter()
        .nth(row_number - 1)
        .expect(&format!("Failed to retrieve {row_number} row"))
        .into_iter()
        .map(|x| x.child_elements().collect())
        .collect();
   
    row.is_empty().not().then(|| row)
}

// TODO: Using Result as return type causes E0515 error...?
//
// /// Returns spans from the column
fn extract_spans_from_column<'a, 'b>(column: &'b Vec<ElementRef<'a>>) -> Result<Vec<Vec<ElementRef<'a>>>, String> {
    let first_element = match column.into_iter().next() {
        Some(el) => el,
        None => return Ok(vec![vec![]]) //return Err(format!("Empty column {column:?}"))
    };

    let span_selector = make_selector(".scp")
        .expect("Failed to make span selector");
    // Each <li> is in own vector
    // For rows that do NOT contain <ul> there is a single vector
    //  containing all spans
    let reqs: Vec< Vec<ElementRef<'a>> > = match first_element.value().name(){
        // <ul> so list of spans
        "ul" => {
            // March through li by li...
            // If a span does not contain a data-skill then wait until the end to see
            // Assign None for now
            // At end take max/min of the set values and apply those to all None
            let li_selector = make_selector("li")
                .expect("Failed to make li selector");
            first_element.select(&li_selector)
                .map(|li| li.child_elements().collect())
                .collect()

            // first_element
            //     .select(&span_selector)
            //     .collect()
        },
        // Spans directly
        _ => {
            vec![column.clone()
                .into_iter()
                .filter(|elementref| elementref.html()
                    .contains("<span")
                ).collect()]
        },
    };

    //dbg!(&reqs);
    Ok(reqs)
}
fn extract_column<'a: 'b, 'b>(row: &'b Vec<Vec<ElementRef<'a>>>, column_number: usize) -> Vec<ElementRef<'a>> {//-> Result<Vec<ElementRef<'a>>, Errors<'a>> {
    assert!(column_number >= 1);

   // TODO: messages
    let column = row.iter().nth(column_number - 1)
        .expect("Failed to retrieve requirements column (2)");

    column.to_vec()
}

#[derive(Debug, Clone)]
struct LevelRequirement {
    pub name: String,
    pub level_list: Vec<u32>, // Empty (or 0) means not set
    pub recommended_list: Vec<bool>,
    pub is_total_level_req_list: Vec<bool> // Is requirement on total level?
}

impl LevelRequirement {
    fn new(name: String, level_list: Vec<u32>, recommended: Vec<bool>, total_level_list: Vec<bool>) -> Self {
        LevelRequirement {name, level_list, 
            recommended_list: recommended,
            is_total_level_req_list: total_level_list
        }
    }

    fn from_span(name: String, level_list_str: Option<&str>) -> Self {
        let name = if name == "Skills" {
            "Total Level".to_string()
        } else { name };

        if level_list_str.is_none() {
            return Self::new(name, Vec::new(), Vec::new(), Vec::new())
        };

        let (level_list, recommended, total_level_list) = Self::parse_span_levels(
            // Can be unchecked since just checked none case...
            level_list_str.unwrap() 
        );
        Self::new(name, level_list, recommended, total_level_list)
    }

    fn get_name(&self) -> String {
        self.name.to_string().to_lowercase()
    }

    fn get_level(&self, strict_recommended: bool) -> u32 {
        self.get_single_level_and_recommended(strict_recommended).0
    }

    fn get_recommended(&self, strict_recommended: bool) -> bool {
        self.get_single_level_and_recommended(strict_recommended).1
    }
    fn get_single_level_and_recommended(&self, strict_recommended: bool) -> (u32, bool) {
        if self.level_list.len() == 0 {
            return (0, false) // Not set
        }
        if self.level_list.len() == 1 {
            return (*self.level_list.first().unwrap(), *self.recommended_list.first().unwrap())
        }


        if strict_recommended {
            let l = zip(&self.level_list, &self.recommended_list);
            let mut recommended_levels: Vec<(&u32, &bool)> = l.filter(|(_, rec)|
                **rec).collect();
            recommended_levels.sort_by(|a, b| (a.0).cmp(b.0)); // Ascending
            let (&lvl, &rec) = recommended_levels.first().unwrap_or_else(|| &(&0, &false));//.expect("This should not be empty...");
            (lvl, rec)
        } else {
            let mut recommended_levels = self.level_list.clone();
            recommended_levels.sort(); // Ascending
            (*recommended_levels.first().unwrap_or_else(|| &0), false) //.expect("This should not be empty..."), false)
        }
    }

    fn parse_level(level_str: &str) -> (u32, bool, bool) {
        // String contains no whitespace
        // Example: 64+Recommended or 64Recommended or 64+ or 64

        // RARELY: 750+ [[Total level]] or 1,500+ [[Total level]]
        // Ignore + as same semantic meaning as nothing
        let level_string: String = level_str.replace("+", "");
        // Whitespace will be cleared in is_empty check after
        
        // FUTURE PROOF: Check if exact string "ecommended" is inside
        // INSTEAD of simply doing is_numeric
        let split_str: Vec<&str> = level_string.split(char::is_whitespace)
            .filter(|s| !s.is_empty())
            .collect();
        let length = split_str.len();
        let mut split_str_iter = split_str.clone().into_iter();
        assert!(length >= 1, "Empty level string after split: {level_str}: {level_string} : {split_str:?}");
        // Parse fails with commas
        //  Always there in iter...hopefully
        let number_str = split_str_iter.next().unwrap().replace(",","");
        
        // Check if number_str contains "high"
        let number = if number_str.contains("igh") {
            // Set a default value
            // 70, 80?
            let high_level = 80;
            high_level

        } else if number_str.contains("ecent") {
            let decent_level = 70;
            decent_level

        } else { match number_str.parse() {
            Ok(n) => n,
            Err(e) => panic!("{e} : {level_str}: {level_string} : {split_str:?} : {number_str}"),//..expect("Level was not a number");

        }};

        match length {
            // No modifiers
            1 => (number, false, false),
            
            // Check for Recommended, Total level
            2 | 3 => {
                let mut recommended: bool = false;
                let mut total_level: bool = false;
                for _ in 1..length {
                    let next_value = split_str_iter.next().unwrap();
                    
                    recommended |= next_value.contains("ecommended");
                    total_level |= next_value.contains("Total level");
                } 

                (number, recommended, total_level)
            },

            _ => unreachable!("Check website. More options have been added: {split_str:?}"),
        }
    }

    /// *For a single skill*
    fn parse_span_levels(level_str: &str) -> (Vec<u32>, Vec<bool>, Vec<bool>) {
        /* Example: Level 64
         * 64 (No modifier)
         * 64+ (At least this level strict/not-strict?)
         * 64 Recommended (Strongly encouraged but not required)
         * 64+ Recommended (Combination of 2 and 3)
         */

        // Treat cases 1. and 2. the same?
        // Recommended should be an option for strict or not-strict level req

        // Or is just a word
        // 70/80+
        let split_levels: String = level_str.replace("or", ",").replace("/",",");
        let split_levels: Vec<&str> = split_levels.split(",")
            .filter(|s| !(s.is_empty() || s == &" ")).collect();
        //dbg!(&split_levels);
        let reqs: Vec<(u32, bool, bool)> = split_levels.into_iter()
            .map(Self::parse_level).collect();
        //dbg!("success");
        let level_reqs: Vec<u32> = reqs.iter().map(|x| x.0).collect();
        let bool_reqs: Vec<bool> = reqs.iter().map(|x| x.1).collect();
        let total_reqs: Vec<bool> = reqs.iter().map(|x| x.2).collect();
        (level_reqs, bool_reqs, total_reqs)
    }
}

impl Display for LevelRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self.get_name();
        let lvl = self.get_level(false);
        write!(f, "{name}, {lvl}")
    }
}

fn config_has_required_levels(config_levels: &Levels, level_reqs: &Vec<LevelRequirement>, strict_recommended: bool) -> bool {
    level_reqs.iter().all(|skill_requirements| {
            let name = skill_requirements.get_name();
            let lvl = skill_requirements.get_level(strict_recommended);
            config_levels.levels.get(&name).expect(&format!("Missing config level: {name} : {skill_requirements}")) >= &lvl
    })
}

/// Returns a vector of skill names and potential requirements of each skill
fn get_requirement_from_span(span: &ElementRef) -> Option<LevelRequirement> {
    let element = span.value();

    let name: Option<String> = match element.attr("data-skill") {
        Some(name) => Some(name.to_string()),
        None => None, // Either messed up or simply an unlock 
                      // (e.g., resurrection spells)
        // TODO: Log None occurences
    };
        //.expect(&format!("No skill name found in span: {span:?}"))
        //.to_string();
    if name.is_none() { return None }
    let level_req_str = element.attr("data-level");
        //.expect(&format!("No skill level attribute found in span: {span:?}"));
    Some(LevelRequirement::from_span(name.unwrap(), level_req_str))
}

fn get_level_from_ul(ul: &Vec<Vec<ElementRef>>, strict_recommended: bool) -> Vec<LevelRequirement> {
    // March through li by li...
    // If a span does not contain a data-skill then wait until the end to see
    // Assign None for now
    // At end take max/min of the set values and apply those to all None


    /* NOTE: This is not exactly how the wiki is written
     * Sometimes you may have: 60, None, None, None, 70
     *      Where None values should be 60
     * Other times you may have: None, None, None, 70, 60
     *      Where None should be 70
     * TODO: IMPLEMENT THIS?!?!?
     *      (Replace functionality of max_exists_level with actual process)
    */
    let mut max_exists_level: u32 = 0; // 0 for unset `LevelRequirement`
    let mut max_recommended: bool = false;
    let mut skill_requirements: Vec<LevelRequirement> = Vec::with_capacity(ul.len());
    for li in ul {
        let mut li_reqs: Vec<LevelRequirement> = Vec::with_capacity(5); // Usually less (~3)

        // First pass to get variables
        // And set max value of min_exists_level
        for span in li {
            let level_req = match get_requirement_from_span(span) {
                Some(level_req) => level_req,
                None => continue // TODO: Don't ignore as may be an unlock
            };

            let level = level_req.get_level(strict_recommended);
            if level != 0 {
                max_exists_level = max_exists_level.max(level);
                max_recommended |= level_req.get_recommended(strict_recommended);
            }

            li_reqs.push(level_req);
        } 


        // Second pass to update 0 values with min_exists_level
        li_reqs = li_reqs.into_iter().map(|mut x| {
            if x.get_level(strict_recommended) == 0 {
                x.level_list = vec![max_exists_level];
                x.recommended_list = vec![max_recommended];
                x.is_total_level_req_list = vec![false];
            }; x 
        }).collect();

        skill_requirements.extend(li_reqs);
    };

    skill_requirements
}


/// TODO: Replace row with a "Method" struct?
fn has_required_levels_for_method(config_levels: &Levels, row: &Vec<Vec<ElementRef>> ) -> bool {
    let level_req_spans = match extract_spans_from_column(&extract_column(&row, 3)) {
       Ok(ul_spans) => ul_spans,
       Err(reason) => panic!("Error at row: {row:?}: {reason}")
    };

    //let level_reqs: Vec<LevelRequirement> = level_req_spans.iter().map(get_level_from_ul).collect();
    let strict_recommended: bool = config_levels.strict_recommended;
    let level_reqs: Vec<LevelRequirement> = get_level_from_ul(&level_req_spans, strict_recommended);
    let has_required = config_has_required_levels(&config_levels, &level_reqs, true);

    has_required
}

fn main() -> Result<(), Errors<'static>> {
    let config: Config = config::load_config("config.yaml");
    let config_levels: Levels = config.levels;

    let url: &str  = "https://oldschool.runescape.wiki/w/Money_making_guide";
    let body: String = retrieve_webpage(url, false)?;
    let html = Html::parse_document(&body);

    // TODO: Using ? here causes E0515 error???
    let table = extract_table(&html, 1).expect("Table was empty");

    //dbg!(&table);
    let rows = (1..=table.len()).map(|row_num|
        extract_row(&table, row_num).expect(&format!("Row {row_num} is empty"))
    ).collect::<Vec<_>>();
    //let row = extract_row(&table, 15).expect("Row is empty");

    //dbg!(&rows);
    // TODO: Name
    let possible_methods_idx: Vec<usize> = rows.iter()
        .enumerate()
        .filter_map(|(idx, row)| 
            (
                has_required_levels_for_method(&config_levels, row)
            ).then(|| idx))
        .collect();

    //dbg!(&possible_methods_idx);

    let possible_methods_rows: Vec<&Vec<Vec<ElementRef>>> = possible_methods_idx
        .iter()
        .map(|&i| rows.get(i).expect(&format!("Row missing at index {i}")))
        .collect();

    //dbg!(&possible_methods_rows);
    let possible_methods_names: Vec<String> = possible_methods_rows
        .iter()
        .map(|row| extract_column(&row, 1).iter()
            .next().expect("Missing method name")
            .value().attr("title").expect("Missing title")
            .replace("Money making guide/", "")
        )
        .collect();

    //dbg!(&possible_methods_names);

    // Write results to a file
    // Overwrite file with new body data
    let path = Path::new("src\\bin\\wiki_info\\wiki_allowed_recipes.txt");
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
        Ok(file) => file,
    };
    file.set_len(0)?;

    for method in possible_methods_names {
        if let Err(e) = writeln!(file, "{method}") {
            panic!("Failed to write line: {}", e);
        }
    } 

    println!("Successfully wrote to {}", path.display());

    // match file.write(possible_methods_names.as_bytes()) {
    //     Err(why) => panic!("couldn't write to {}: {}", path.display(), why),
    //     Ok(_) => println!("successfully wrote to {}", path.display()),
    // }
    file.sync_all().expect("Failed to sync data to filesystem");


    Ok(())
}
