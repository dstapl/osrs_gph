use crate::{
    config::{Membership, OverviewFilter, OverviewSortBy},
    helpers::f_round,
    item_search::{
        item_search::{Item, ItemSearch},
        recipes::{Recipe, RecipeBook, RecipeTime},
    },
    types::{DetailedTable, TableInputs, OverviewRow, SEC_IN_HOUR},
};

use std::collections::HashMap;

use super::pareto_sort::custom_types::{optimal_sort, Weights};

use tracing::{debug, warn};

// TODO: CHANGE TO ACTUAL TYPES
pub type Row = (i32, i32, i32, RecipeTime);

// TODO: Remove when changing types
// type _TableRow = (String, String, String, String, String);
/// `recipe_name`, money, total money, total time, gp/h
pub type _TableRow = [String; 5];

pub type Table = Vec<_TableRow>; // Vec of parsed Rows

pub struct PriceHandle {
    pub all_items: ItemSearch,
    pub recipe_list: RecipeBook,
    pub coins: i32,
    pub pmargin: f32,
}

use crate::types::SECOND_PER_TICK;

// NOTE: Copy implemented since only simple enum variants
#[derive(Debug, Default, Clone, Copy)]
pub enum TimeType {
    SingleHour,
    #[default]
    MaxHours,
}


/// # Panics
/// Panics if `single_cost` is negative. But this shouldn't be implemented in practice
pub fn update_recipe_number(number_per_hour: Option<i32>, coins: i32, single_cost: i32, time_type: TimeType) -> i32 {
    // assert!( number_per_hour.is_some() || single_cost != 0, "{number_per_hour:?} {single_cost}");
    if number_per_hour.is_none() && single_cost == 0 {
        // No per-hour info and no cost -> return a safe default of 1.
        // Change this to `i32::MAX` if you prefer "unbounded".
        return 0;
    }
    
    assert!(single_cost >= 0, "Cost of recipe is negative?");
    
    let no_cost = single_cost == 0;

    let val_per_hour = number_per_hour.unwrap_or_else(|| {
            // `no_cost` MUST be false from assertions
            //  so div by zero should NOT occur
            // Estimate the total number possible from given coins
            let effective_max_from_coins: i32 = coins.saturating_div(single_cost);
            effective_max_from_coins
        }
    );

    if matches!(time_type, TimeType::SingleHour) || no_cost || number_per_hour.is_none() {
        return val_per_hour;
    }

    let number_per_hour = val_per_hour;

    // Calculate the highest number of hours as a multiple of number_per_hour
    let cost_per_hour_i64: i64 = i64::from(single_cost).saturating_mul(i64::from(number_per_hour));
    let coins_i64 = i64::from(coins);
    let single_cost_i64 = i64::from(single_cost);

    let full_hours = coins_i64.saturating_div(cost_per_hour_i64);
    #[allow(clippy::cast_possible_truncation)]
    let mut total_number = (full_hours as i32).saturating_mul(number_per_hour);

    // Calculate any fractional time lost through integer truncation
    let remainder_time = coins_i64.saturating_sub(full_hours.saturating_mul(cost_per_hour_i64));
    if remainder_time >= single_cost_i64 {
        #[allow(clippy::cast_possible_truncation)]
        let extra_time = remainder_time.saturating_div(single_cost_i64) as i32;
        total_number = total_number.saturating_add(extra_time);
    }

    // Know `no_cost` must be false at this point
    let effective_max_from_coins: i32 = coins.saturating_div(single_cost);

    total_number.min(effective_max_from_coins)
    .max(1)
}

impl PriceHandle {
    pub fn new(all_items: ItemSearch, recipe_list: RecipeBook, coins: i32, pmargin: f32) -> Self {
        Self {
            all_items,
            recipe_list,
            coins,
            pmargin,
        }
    }

    // TODO: Change price_options to a struct; like FileOptions?
    /// Display recipe overview for every recipe recorded in memory
    /// # Panics
    /// Will panic if the recipe list is empty.
    /// Refer to `filepaths/lookup_data/recipes` in [`config.yaml`]
    pub fn all_recipe_overview(
        &self,
        sort_by_option: &OverviewSortBy,
        sort_by_weights: &Weights,
        price_options: &crate::config::Display,
    ) -> Vec<OverviewRow> {
        let profiting = price_options.filters[OverviewFilter::MustProfit];
        let show_hidden = price_options.filters[OverviewFilter::ShowHidden];
        let reverse = price_options.filters[OverviewFilter::Reverse];
        let membership_option = &price_options.membership;

        // Get recipe input/output prices
        let recipe_list = self.recipe_list.get_all_recipes();
        assert!(!recipe_list.is_empty());

        let time_type = price_options.time_type;

        let all_recipe_prices = recipe_list
            .keys()
            .filter_map(|recipe_name| {
                let overview_output = self.recipe_price_overview_from_string(recipe_name, time_type)?;
                Some((recipe_name, overview_output))
            })
            .collect::<HashMap<_, _>>();

        assert!(!all_recipe_prices.is_empty());

        // Construct details
        let mut all_overviews = Vec::new();
        let coins = self.coins;

        for (recipe_name, (mut overview, (cost, _revenue))) in all_recipe_prices {
            let needs_members = recipe_list[recipe_name].members;

            let skip = match membership_option {
                Membership::F2P => needs_members,
                Membership::P2P => !needs_members,
                Membership::BOTH => false,
            };

            if skip {
                debug!(
                    desc = "Skipping recipe for membership requirement...",
                    name = %recipe_name,
                    config = ?membership_option,
                    recipe = %needs_members
                );
                continue;
            }

            // let profit = overview.profit;
            let profit = overview.loss_gain();
            let recipe_cost = cost;

            let cant_afford = coins < recipe_cost;
            let no_profit = profit <= 0;

            // TODO: Assign some boolean values to variables so reused?
            // Or leave to the compiler instead?

            // Used Karnaugh map to calculate
            if (cant_afford && !show_hidden) || (no_profit && profiting && !show_hidden) {
                debug!(desc = "Skipping recipe from Karnaugh map...", name = %recipe_name);
                continue;
            }

            // Add modifier to show not profiting
            if (cant_afford && show_hidden) || (no_profit && profiting && show_hidden) {
                // TODO: Better differentiate this
                // Add modifier to the recipe name ?
                // Change colour?
                overview.name += " *";
            }

            all_overviews.push(overview);
        }


        // Sort based on option selected in config
        match sort_by_option {
            // TODO: Way other than clone? Even though strings are short-ish
            OverviewSortBy::Name => {
                all_overviews.sort_by_key(|k| k.name.clone());
                if reverse {
                    all_overviews.reverse();
                }
            },
            // TODO: Are these the same order as total times / total profit?
            OverviewSortBy::Profit => {
                #[allow(clippy::redundant_closure_for_method_calls)]
                all_overviews.sort_by_key(|k| k.total_gp());
                if !reverse { // Highest profit first
                    all_overviews.reverse();
                }
            },
            OverviewSortBy::Time => {
                #[allow(clippy::cast_possible_truncation)]
                all_overviews.sort_by_key(|k| (k.total_time().unwrap_or(f32::MAX) * f32::from(SEC_IN_HOUR) ) as i32);
                if reverse {
                    all_overviews.reverse();
                }
            },
            OverviewSortBy::GPH => {
                #[allow(clippy::redundant_closure_for_method_calls)]
                all_overviews.sort_by_key(|k| k.gph());

                if !reverse { // Highest GP/h first
                    all_overviews.reverse();
                }
            },
            // TODO: Does this actually change the order of the rows?
            OverviewSortBy::Custom => optimal_sort(&mut all_overviews, sort_by_weights, !reverse),
        }

        all_overviews
    }

    pub fn recipe_lookup_from_recipe(&self, recipe: &Recipe, time_type: TimeType) -> Option<DetailedTable> {
        // Need to parse item strings into Item objects
        // debug!(desc = "Parsing recipe lookup", name = &recipe.name);
        let pay_once_items: Option<Vec<_>> = recipe.inputs.pay_once.as_ref()
            .and_then(|items| self.parse_item_list(items));
        let input_items = self.parse_item_list(&recipe.inputs.inputs)?;
        let output_items = self.parse_item_list(&recipe.outputs)?;

        // HashMap[item -> (price, quantity)]
        // Base price
        let pay_once_details = pay_once_items.map(|items| PriceHandle::item_list_prices_unchecked(items, true));
        let input_details = PriceHandle::item_list_prices_unchecked(input_items, true);

        let output_details = PriceHandle::item_list_prices_unchecked(output_items, false);

        let (overview, (_,_)) = self.recipe_price_overview_from_recipe(recipe, time_type)?;

        // Form table
        // Transform input/outputs to DetailedTable type
        // TODO: Switch DetailedTable input/outputs types to HashMap instead
        let pay_once_vec = pay_once_details.map(|details|
            details.into_iter()
                .map(|(item, (price, quantity))| (item.name,price,quantity))
                .collect()
        );
        let input_vec = input_details.into_iter()
            .map(|(item, (price, quantity))| (item.name,price,quantity))
            .collect();

        let table_inputs = TableInputs {
            pay_once: pay_once_vec,
            inputs: input_vec
        };


        let output_vec = output_details.into_iter()
            .map(|(item, (price, quantity))| (item.name,price,quantity))
            .collect();

        let recipe_lookup: DetailedTable = DetailedTable::new(
            overview,
            table_inputs,
            output_vec, // Not taxed
            self.pmargin,
        );

        Some(recipe_lookup)
    }

    pub fn recipe_price_overview_from_string(&self, recipe_name: &String, time_type: TimeType) -> Option<(OverviewRow, (i32, i32))>  {
        let recipe = self.recipe_list.get_recipe(recipe_name)?;
        self.recipe_price_overview_from_recipe(recipe, time_type)
    }

    
    fn calculate_buy_limit_item(input_details: &HashMap<Item, (i32, f32)>)
        -> Option<(Item, i32)> {
        if input_details.is_empty() {
            return None // No input items required
        }

        let mut limit_item_number: Option<(Item, i32)> = None;

        for (item, (_, quantity)) in input_details {
            let buy_limit = item.limit.unwrap_or(i32::MAX);

            #[allow(clippy::cast_possible_truncation)]
            let number = 
                (f64::from(buy_limit)/f64::from(*quantity))
                .floor() as i32;

            match limit_item_number.as_mut() {
                // Initialise value
                None => limit_item_number = Some((item.to_owned(), number)),
                // Update if the number is smaller
                Some((limit_item, limit_number)) => if &number < limit_number {
                    item.clone_into(limit_item);
                    *limit_number = number;
                }
            }

        };

        limit_item_number
    }


    /// Returns price overview and cost of inputs and (taxed) revenue from outputs
    #[allow(clippy::missing_panics_doc, reason = "infallible")]
    pub fn recipe_price_overview_from_recipe(&self, recipe: &Recipe, time_type: TimeType) -> Option<(OverviewRow, (i32, i32))> {
        // Need to parse item strings into Item objects
        let pay_once_items: Option<Vec<_>> = recipe.inputs.pay_once.as_ref()
            .and_then(|items| self.parse_item_list(items));
        let input_items = self.parse_item_list(&recipe.inputs.inputs)?;

        let output_items = self.parse_item_list(&recipe.outputs)?;

        let pay_once_details = pay_once_items.map(|items| PriceHandle::item_list_prices_unchecked(items, true));
        let input_details = PriceHandle::item_list_prices_unchecked(input_items, true);
        // assert!(!input_details.is_empty());

        let output_details = PriceHandle::item_list_prices_unchecked(output_items, false);
        assert!(!output_details.is_empty());


        // Minimum of (max_buy_limit / item_number_in_recipe) for all inputs
        let item_limit_number = PriceHandle::calculate_buy_limit_item(
            &input_details
        );


        let pay_once_cost = pay_once_details.as_ref().map(|details|
            PriceHandle::total_details_price(
                &details.values().copied().collect::<Vec<_>>(),
                false
            )
        );


        let revenue = PriceHandle::total_details_price(
            &output_details.into_values().collect::<Vec<_>>(),
            true
        );
        let cost = PriceHandle::total_details_price(
            &input_details.into_values().collect::<Vec<_>>(),
            false
        );
        let profit = revenue - cost;


        let time_ticks = recipe.ticks.clone();
        // Return None from function if Invalid
        let time_sec = match time_ticks {
            RecipeTime::INVALID => None, // Could be N/A e.g. slayer task
            RecipeTime::Time(recipe_time) => Some(recipe_time * SECOND_PER_TICK)
        };


        // Stay None if time_sec is undefined
        let user_number_per_hour = recipe.number_per_hour;


        // TODO(1): Choice to include this calculation or not? When None.
        // Would simplify logic a lot, since only considering f64 not Option<f64>
        let mut effective_time_sec: Option<f64> = None;

        if time_sec.is_some() {
            effective_time_sec = time_sec.map(f64::from);
        } else { // None
            debug!(desc = "RecipeTime is not set.", name = %recipe.name);
            if user_number_per_hour.is_none() {
                warn!(desc = "RecipeTime AND number_per_hour are not set.", name = %recipe.name);
                return None; // No valid values for calculating time so exit function
            }

            // Otherwise override effective_time_sec with user_number calculation
            let user_eff_time = 60.0f64 * 60.0f64 / f64::from(user_number_per_hour.expect("Just checked user_number_per_hour is not none...?"));
            effective_time_sec = Some(match effective_time_sec {
                None => user_eff_time,
                Some(time) => time.max(user_eff_time)
            });
        }


        #[allow(clippy::cast_possible_truncation)]
        let effective_number_per_single_hour = effective_time_sec
            .map(|f| (60.0 * 60.0 / f).ceil() as i32);

        let number_per_hour = match time_type {
            TimeType::SingleHour => effective_number_per_single_hour,
            TimeType::MaxHours => user_number_per_hour,
        };
        
        // One or more of time or user_number_per_hour is set
        let mut number = update_recipe_number(number_per_hour, self.coins, cost, time_type);

        if let Some((item, limit_number)) = item_limit_number {
            if limit_number < number {
                debug!(
                    recipe = &recipe.name,
                    item_limiter = &item.name,
                    limit = &limit_number
                );

                // Update number since restricted by buy limit
                number = limit_number;
            }
        }

        #[allow(clippy::items_after_statements)]
        const MAX_HOURS: f64 = 6.0;

        if let Some(eff_time_sec) = effective_time_sec {
            // Allowed number of recipes so total_time <= MAX_HOURS
            #[allow(clippy::cast_possible_truncation)]
            let max_allowed = ((MAX_HOURS * 3600.0) / eff_time_sec).floor() as i32;
            if max_allowed > 0 && number > max_allowed {
                debug!(
                    desc = "Capping recipe number to respect the log-out timer",
                    recipe = %recipe.name,
                    old_number = number,
                    capped_to = max_allowed
                );
                number = max_allowed;
            }
        }
        number = number.max(1);

        #[allow(clippy::cast_possible_truncation)]
        let overview_single_time = effective_time_sec.map(|f| f as f32);

        let overview = OverviewRow::new(
            recipe.name.clone(),
            pay_once_cost,
            profit,
            overview_single_time,
            number,
            time_type
        );

        Some((overview, (cost, revenue)))
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn apply_tax(profit: i32) -> i32 {
        const TAX_PERCENT: f64 = 2.0;
        const FEE_CAP: i32 = 5_000_000;

        // Update to 2% tax 2025-05-29
        // https://oldschool.runescape.wiki/w/Grand_Exchange#Convenience_fee_and_item_sink
        if profit < 50 {
            return profit;
        }
        

        #[allow(clippy::cast_possible_truncation)]
        // SAFETY: original values and multiplication are within i32 limit
        let untaxed = (f64::from(profit) * TAX_PERCENT / 100.0).floor() as i32;
        let tax: i32 = FEE_CAP.min(untaxed);

        profit - tax
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn total_details_price(price_details: &[(i32, f32)], apply_tax: bool) -> i32 {
        // total price for each item is price * quantity
        let price_totals = price_details.iter().map(|t| f64::from(t.0) * f64::from(t.1));
        
        #[allow(clippy::cast_possible_truncation)]
        return if apply_tax {
            price_totals.map(|f| f as i32)
                .map(PriceHandle::apply_tax)
                .sum::<i32>()
        } else {
            price_totals.sum::<f64>().floor() as i32
        }

        // // SAFETY: Max size of i32 < f64
        // return total.floor() as i32
    }

    #[allow(clippy::cast_precision_loss)]
    /// Returns total time in hours, and estimated GP/hour
    pub fn recipe_time_h(time: f32, number: i32, margin: i32, total_margin: bool) -> (f32, i32) {
        let time_h: f32 = time / (60. * 60.);
        let total_time_h: f32 = f_round(number as f32 * time_h, 2);

        // SAFETY: f64 can represent all values of i32
        // TODO: times should not be small enough to surpass accuracy limit?
        #[allow(clippy::cast_possible_truncation)]
        let gp_h = if total_margin {
            (f64::from(margin) / f64::from(total_time_h)).floor() as i32
        } else {
            (f64::from(margin) / f64::from(time_h)).floor() as i32
        };

        (total_time_h, gp_h)
    }

    /// (Item, (Price, Quantity))
    pub fn item_list_prices<I: IntoIterator<Item = (Item, f32)>>(
        item_list: I,
        price_type: bool,
    ) -> HashMap<Item, (Option<i32>, f32)> {
        item_list
            .into_iter()
            .map(|(i, q)| {
                let price = i.price(price_type);
                (i, (price, q))
            })
            .collect()
    }

    /// # Panics
    /// When item price does not exist.
    /// true means buy, false means sell
    /// # Returns
    /// `HashMap` (Item, (Price, Quantity))
    pub fn item_list_prices_unchecked<I: IntoIterator<Item = (Item, f32)>>(
        item_list: I,
        price_type: bool,
    ) -> HashMap<Item, (i32, f32)> {
        item_list
            .into_iter()
            .map(|(i, q)| {
                let price = i.price(price_type).unwrap();
                (i, (price, q))
            })
            .collect()
    }

    pub fn parse_item_list(&self, item_list: &HashMap<String, f32>) -> Option<Vec<(Item, f32)>> {
        // TODO: Compare methods of take_while (then re-iter) vs filter_map
        let filtered_items: Vec<(Item, f32)> = item_list
            .iter()
            // Stop if encounter None value rather than processing whole list
            .map_while(|(item_name, &quantity)| {
                self.all_items
                    .item_by_name(item_name)
                    .map(|item_option| (item_option.clone(), quantity))
            })
            .collect();

        if item_list.len() == filtered_items.len() {
            // SAFETY: Know all elements are in lookup and are type Item
            Some(filtered_items)
        } else {
            let original_names = item_list.keys();
            let new_names: Vec<_> = filtered_items.
                iter().map(|(item, _)| item.name.clone())
                .collect();
            let difference: Vec<_> = original_names
                .clone()
                .filter(|name| !new_names.contains(name))
                .collect();
            warn!(desc = "Items not found in item_list lookup",
                diff = ?difference, old = ?original_names, new = ?new_names
            );
            None
        }
    }
}
