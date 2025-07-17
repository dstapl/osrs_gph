# General
- [ ] **Revise i32, f32, f64 types** when multiplying; i32::MAX << f32::MAX
    - [ ] Turn this into a function? 
        .map(|(_,price,quantity)| (f64::from(*price) * f64::from(*quantity)) as i32).sum::<i32>()

- [ ] Separate the front & back-end
- [ ] Change markdown output to a legacy feature
- [ ] Implement a rigorous sorting function for `custom` option in
  OverviewSortBy
- [ ] Allow filtering by method type (Match from Wiki)
    - [ ] Add attribute for method type in Recipe Struct
- [ ] Add error message for when an item name is not found on parsing
    - [ ] Warn that lookups may need to be regenerated
# Web migration
- [ ] Create html/css(/js?) mockup
- [ ] Use Leptos framework in Rust

# Logging
- [x] runtime.log file not being cleared when the number of recipes is lower (table isn't as long)
    - [x] Check the sync_all function
- [ ] Change logging macros to correct types: debug -> trace where applicable

# API/Price Data
- [ ] Store previous price data in a database
    - [ ] Store last (n) prices for each item with associated date-times
    - [ ] Visualise historic price data (from storage)

# /bin/*

## Generate Lookups

## Wiki pages
- [ ] Auto-generate recipes from Wiki pages
    - [ ] Parse inputs and outputs
    - [ ] Include requirements or other? E.g. quest points
    - [ ] Parse kills/hour into a time and number of outputs
        -
    - [ ] Auto-update lookup_data/recipes.yaml with parsed recipes