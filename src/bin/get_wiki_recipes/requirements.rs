use std::fmt::Display;
use std::iter::zip;

#[derive(Debug, Clone)]
pub struct LevelRequirement {
    pub name: String,
    // TODO: Technically non-negative i32
    pub level_list: Vec<u32>, // Empty (or 0) means not set
    pub recommended_list: Vec<bool>,
    pub is_total_level_req_list: Vec<bool>, // Is requirement on total level?
}

#[derive(Debug)]
pub struct MoneyMethod {
    pub name: String,
    pub requirements: Vec<LevelRequirement>,
}

impl LevelRequirement {
    pub fn new(
        name: String,
        level_list: Vec<u32>,
        recommended: Vec<bool>,
        total_level_list: Vec<bool>,
    ) -> Self {
        LevelRequirement {
            name,
            level_list,
            recommended_list: recommended,
            is_total_level_req_list: total_level_list,
        }
    }

    pub fn from_span(name: String, level_list_str: Option<&str>) -> Self {
        let name = if name == "Skills" {
            "Total Level".to_string()
        } else { name };


        if level_list_str.is_none() {
            return Self::new(name, Vec::new(), Vec::new(), Vec::new());
        }

        let (level_list, recommended, total_level_list) = Self::parse_span_levels(
            // Can be unchecked since just checked the None case...
            level_list_str.unwrap(),
        );
        Self::new(name, level_list, recommended, total_level_list)
    }

    pub fn get_name(&self) -> String {
        self.name.to_string().to_lowercase()
    }

    pub fn get_level(&self, strict_recommended: bool) -> u32 {
        self.get_single_level_and_recommended(strict_recommended).0
    }

    pub fn get_recommended(&self, strict_recommended: bool) -> bool {
        self.get_single_level_and_recommended(strict_recommended).1
    }
    pub fn get_single_level_and_recommended(&self, strict_recommended: bool) -> (u32, bool) {
        if self.level_list.is_empty() {
            return (0, false); // Not set
        }
        if self.level_list.len() == 1 {
            return (
                *self.level_list.first().unwrap(),
                *self.recommended_list.first().unwrap(),
            );
        }

        // <= inequality check
        if !strict_recommended {
            let mut recommended_levels = self.level_list.clone();
            recommended_levels.sort_unstable(); // Ascending
            return (*recommended_levels.first().unwrap_or(&0), false) //.expect("This should not be empty..."), false)
        } 


        // < inequality check; Strict recommended
        let l = zip(&self.level_list, &self.recommended_list);

        let mut recommended_levels: Vec<(&u32, &bool)> = l.filter(|(_, rec)| **rec).collect();
        recommended_levels.sort_by(|a, b| (a.0).cmp(b.0)); // Ascending

        let &(&lvl, &rec) = recommended_levels.first().unwrap_or(&(&0, &false)); //.expect("This should not be empty...");
        (lvl, rec)
    }

    pub fn parse_level(level_str: &str) -> (u32, bool, bool) {
        // String contains no whitespace
        // Example: 64+Recommended or 64Recommended or 64+ or 64

        // RARELY: 750+ [[Total level]] or 1,500+ [[Total level]]
        // Ignore + as same semantic meaning as nothing
        let level_string: String = level_str.replace('+', "");
        // Whitespace will be cleared in is_empty check after

        // FUTURE PROOF: Check if exact string "ecommended" is inside
        // INSTEAD of simply doing is_numeric
        let split_str: Vec<&str> = level_string
            .split(char::is_whitespace)
            .filter(|s| !s.is_empty())
            .collect();
        let length = split_str.len();
        let mut split_str_iter = split_str.clone().into_iter();
        assert!(
            length >= 1,
            "Empty level string after split: {level_str}: {level_string} : {split_str:?}"
        );
        // Parse fails with commas
        //  Always there in iter...hopefully
        let number_str = split_str_iter.next().unwrap().replace(',', "");

        // Check if number_str contains "high"
        let number = if number_str.contains("igh") {
            // Set a default value
            // 70, 80?

            80
        } else if number_str.contains("ecent") {
            70
        } else {
            match number_str.parse() {
                Ok(n) => n,
                Err(e) => {
                    panic!("{e} : {level_str}: {level_string} : {split_str:?} : {number_str}")
                } //..expect("Level was not a number");
            }
        };

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
            }

            _ => unreachable!("Check website. More options have been added: {split_str:?}"),
        }
    }

    /// *For a single skill*
    pub fn parse_span_levels(level_str: &str) -> (Vec<u32>, Vec<bool>, Vec<bool>) {
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
        let split_levels: String = level_str.replace("or", ",").replace('/', ",");
        let split_levels: Vec<&str> = split_levels
            .split(',')
            .filter(|s| !(s.is_empty() || s == &" "))
            .collect();
        //dbg!(&split_levels);
        let reqs: Vec<(u32, bool, bool)> =
            split_levels.into_iter().map(Self::parse_level).collect();
        //dbg!("success");

        let level_reqs: Vec<u32> = reqs.iter().map(|x| x.0).collect();
        let bool_reqs: Vec<bool> = reqs.iter().map(|x| x.1).collect();
        let total_reqs: Vec<bool> = reqs.iter().map(|x| x.2).collect();
        (level_reqs, bool_reqs, total_reqs)
    }
}

impl Display for LevelRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = self.get_name();
        let lvl = self.get_level(false);
        write!(f, "{name}, {lvl}")
    }
}

