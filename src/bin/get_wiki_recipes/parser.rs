use std::sync::LazyLock;
use scraper::{ElementRef, Selector};
use crate::requirements::{LevelRequirement, MoneyMethod};
use osrs_gph::config::Levels;

// Re-exports
pub use scraper::html::Html;


// TODO: Remove pub where not needed
// Type Aliases
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct TableColumn<'a>(pub Vec<ElementRef<'a>>);

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Span<'a>(pub Vec<ElementRef<'a>>);

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct TableRow<'a>(pub Vec<TableColumn<'a>>);

pub type Table<'a> = Vec<TableRow<'a>>;



// CSS Selectors
static TABLE_SELECTOR: LazyLock<Selector> = LazyLock::new(|| 
    Selector::parse(".wikitable").expect("Invalid table selector")
);
static LI_SELECTOR: LazyLock<Selector> = LazyLock::new(|| 
    Selector::parse("li").expect("Invalid li selector")
);



pub fn extract_table(html: &Html, table_number: usize) -> Option<Table<'_>> {
    assert!(table_number >= 1);

    let table_el = html
        .select(&TABLE_SELECTOR)
        .nth(table_number - 1)
        .expect("Failed to find table {table_number} in HTML");

    let body_el = table_el
        .child_elements()
        .nth(1)?; // Skip caption

    let table: Table = body_el
        .child_elements()
        .skip(1)
        .map(|row| {
            TableRow(
                row.child_elements()
                    .map(|cell| TableColumn(cell.child_elements().collect()))
                    .collect(),
            )
        })
        .filter(|row| !row.0.is_empty()) // Remove empty rows
        .collect();

    // If all rows are empty return None
    (!table.is_empty()).then_some(table)
}

pub fn extract_row<'a>(
    table: &Table<'a>,
    row_number: usize,
) -> Option<TableRow<'a>> {
    assert!(row_number >= 1);

    table
        .get(row_number - 1)
        .cloned()
        .filter(|row| !row.0.is_empty())
}

/// Returns spans from the column
pub fn extract_spans_from_column<'a>(
    column: &TableColumn<'a>,
) -> Option<Vec<Span<'a>>> {
    let first_element = column.0.first()?;

    // Each <li> is in own vector
    // For rows that do NOT contain <ul> there is a single vector
    //  containing all spans
    let reqs: Vec<Span> = match first_element.value().name() {
        // <ul> so list of spans
        "ul" => {
            // March through li by li...
            // If a span does not contain a data-skill then wait until the end to see
            // Assign None for now
            // At end take max/min of the set values and apply those to all None
            first_element
                .select(&LI_SELECTOR)
                .map(|li| Span(li.child_elements().collect()))
                .collect()
        }
        // Spans directly
        _ => {
            vec![Span(
                column
                .clone()
                .0.into_iter()
                .filter(|elementref| elementref.html().contains("<span"))
                .collect()
            )]
        }
    };

    Some(reqs)
}

// Extract a column from an index of a row.
// NOTE: May return an empty column
pub fn extract_column<'a>(
    row: &TableRow<'a>,
    column_number: usize,
) -> TableColumn<'a> {
    assert!(column_number >= 1);

    // TODO: messages
    let column = row.0
        .get(column_number - 1)
        .expect("Failed to retrieve requirements column (2)");

    column.clone()
}


/// Returns a vector of skill names and potential requirements of each skill
fn get_requirement_from_span(span: &ElementRef) -> Option<LevelRequirement> {
    let element = span.value();

    let name: Option<String> = element
        .attr("data-skill")
        .map(std::string::ToString::to_string);
    //.expect(&format!("No skill name found in span: {span:?}"))
    //.to_string();
    name.as_ref()?;
    let level_req_str = element.attr("data-level");
    //.expect(&format!("No skill level attribute found in span: {span:?}"));
    Some(LevelRequirement::from_span(name.unwrap(), level_req_str))
}



fn config_has_required_levels(
    config_levels: &Levels,
    level_reqs: &[LevelRequirement],
    strict_recommended: bool,
) -> bool {
    level_reqs.iter().all(|skill_requirements| {
        let name = skill_requirements.get_name();
        let lvl = skill_requirements.get_level(strict_recommended);
        
        config_levels.levels
            .get(&name)
            .expect("Missing config level: {name} : {skill_requirements}")
            .ge(&lvl)
    })
}



fn get_level_from_ul(ul: &Vec<Span>, strict_recommended: bool) -> Vec<LevelRequirement> {
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
        for span in &li.0 {
            let Some(level_req) = get_requirement_from_span(span) else {
                // TODO: Don't ignore as may be an unlock
                continue
            };

            let level = level_req.get_level(strict_recommended);
            if level != 0 {
                max_exists_level = max_exists_level.max(level);
                max_recommended |= level_req.get_recommended(strict_recommended);
            }

            li_reqs.push(level_req);
        }

        // Second pass to update 0 values with min_exists_level
        li_reqs = li_reqs
            .into_iter()
            .map(|mut x| {
                if x.get_level(strict_recommended) == 0 {
                    x.level_list = vec![max_exists_level];
                    x.recommended_list = vec![max_recommended];
                    x.is_total_level_req_list = vec![false];
                }
                x
            })
            .collect();

        skill_requirements.extend(li_reqs);
    }

    skill_requirements
}

/// TODO: Replace row with a "Method" struct?
pub fn has_required_levels_for_method(config_levels: &Levels, row: &TableRow) -> bool {
    let level_req_spans = match extract_spans_from_column(&extract_column(row, 3)) {
        Some(spans) => spans,
        None => return true, // No requirements so always allowed
    };

    let strict_recommended: bool = config_levels.strict_recommended;
    let level_reqs: Vec<LevelRequirement> = get_level_from_ul(&level_req_spans, strict_recommended);

    config_has_required_levels(config_levels, &level_reqs, true)
}

