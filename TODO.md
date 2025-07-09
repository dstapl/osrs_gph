# General
- [ ] **Revise i32, f32, f64 types** when multiplying; i32::MAX << f32::MAX
    - [ ] Turn this into a function? 
        .map(|(_,price,quantity)| (f64::from(*price) * f64::from(*quantity)) as i32).sum::<i32>()

- [ ] Separate the front & back-end
- [ ] Change markdown output to a legacy feature
- [ ] Implement a rigorous sorting function for `custom` option in
  OverviewSortBy

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
