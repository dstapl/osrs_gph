use crate::{
    helpers::{f_round, floor},
    item_search::{
        item_search::{Item, ItemSearch},
        recipes::{RecipeBook, RecipeTime, Recipe},
    }
};

use std::{collections::HashMap, path::Path, fmt::Display};

use super::pareto_sort::custom_types::{Weights, optimal_sort};

use tracing::{warn, debug, trace, info};

use crate::helpers::number_to_comma_sep_string;

// TODO: CHANGE TO ACTUAL TYPES
pub type Row = (i32, i32, i32, RecipeTime);

// TODO: Remove when changing types
// type _TableRow = (String, String, String, String, String);
/// recipe_name, money, total money, total time, gp/h
pub type _TableRow = [String; 5];

pub type Table = Vec<_TableRow>; // Vec of parsed Rows

pub struct PriceHandle {
    pub all_items: ItemSearch,
    pub recipe_list: RecipeBook,
    pub coins: i32,
    pub pmargin: f32,
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

    /// Display recipe overview for every recipe recorded in memory
    // TODO: Change price_options to a struct; like FileOptions?
    pub fn all_recipe_overview(&self, sort_by_weights: &Weights, price_options: &crate::config::Display) -> Table {
        // let [profiting, show_hidden, reverse] = price_options;
        let profiting = price_options.must_profit;
        let show_hidden = price_options.show_hidden;
        let reverse = price_options.reverse;

        let recipe_list = self.recipe_list.get_all_recipes();
        assert!(!recipe_list.is_empty());

        let all_recipe_prices = recipe_list.keys()
            .filter_map(|recipe_name| {
                let overview = self.recipe_price_overview_from_string(recipe_name)?;
                Some((recipe_name, overview))
            }
            ).collect::<HashMap<_,_>>();
        assert!(!all_recipe_prices.is_empty());

        let mut all_recipe_details = Table::new();

        let coins = self.coins.clone();
        for (recipe_name, overview) in all_recipe_prices{
            if !overview.3.isvalid() {
                warn!(desc = "INVALID RecipeTime", name = %recipe_name);
                continue
            };

            let crate::item_search::recipes::RecipeTime::Time(time) = overview.3 else {
                unreachable!("INVALID RecipeTime should already be checked");
            };

            let [recipe_cost_f, margin_f, time] = [overview.0 as f32, overview.2 as f32, time];

            let margin = floor(f64::from(margin_f));
            let recipe_cost = floor(f64::from(recipe_cost_f));

            let cant_afford = coins < recipe_cost;
            let no_profit = margin <= 0;

            // TODO: Assign some boolean values to variables so reused?
            // Or leave to the compiler instead?

            // Used Karnaugh map to calculate
            if  (cant_afford && !show_hidden) || (no_profit && profiting && !show_hidden) {
                debug!(desc = "Skipping recipe from Karnaugh map...", name = %recipe_name);
                continue;
            }

            let [rn_s, m_s, totm_s, tt_s, gph_s] = if (cant_afford && show_hidden) || (no_profit && profiting && show_hidden) {
                [
                    recipe_name.to_owned(),
                    "#".to_owned(),
                    "#".to_owned(),
                    "#".to_owned(),
                    "#".to_owned()
                ]
            } else {
                let amount = floor(f64::from(coins)/f64::from(recipe_cost));

                let (total_time_h, gp_h) = PriceHandle::recipe_time_h(
                    time, amount, margin, false
                );
                [
                    recipe_name.to_owned(),
                    number_to_comma_sep_string(&margin),
                    number_to_comma_sep_string(&(amount*margin)),
                    total_time_h.to_string(),
                    number_to_comma_sep_string(&gp_h)
                ]
            };

            let row: _TableRow = [rn_s, m_s, totm_s, tt_s, gph_s];

            all_recipe_details.push(row);
        }

        // TODO: Does this actually modify?
        optimal_sort(&all_recipe_details, sort_by_weights, reverse)
        // all_recipe_details
    }

    pub fn recipe_price_overview_from_string(&self, recipe_name: &String) -> Option<Row> {
        let recipe = self.recipe_list.get_recipe(recipe_name)?;
        self.recipe_price_overview_from_recipe(recipe)
    }

    pub fn recipe_price_overview_from_recipe(&self, recipe: &Recipe) -> Option<Row> {
        // Need to parse item strings into Item objects
        let input_items = self.parse_item_list(&recipe.inputs)?;
        let output_items = self.parse_item_list(&recipe.outputs)?;

        let input_details = PriceHandle::item_list_prices_unchecked(
            input_items, true
        );
        assert!(!input_details.is_empty());
        
        let output_details = PriceHandle::item_list_prices_unchecked(
            output_items, false
        );
        assert!(!output_details.is_empty());

        let cost = PriceHandle::total_details_price(
            &input_details.into_values().collect::<Vec<_>>()
        );


        let revenue = PriceHandle::apply_tax(
            PriceHandle::total_details_price(
                &output_details.into_values().collect::<Vec<_>>()
            )
        );

        
        let profit = revenue - cost;

        let time = recipe.ticks.clone();

        Some(
            (
            cost,
            revenue,
            profit,
            time
            )
        )
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn apply_tax(profit: i32) -> i32 {
        // Update to 2% tax 2025-05-29
        // https://oldschool.runescape.wiki/w/Grand_Exchange#Convenience_fee_and_item_sink
        if profit < 50 {
            profit
        } else {
            const TAX_PERCENT: f64 = 2.0;
            const FEE_CAP: i32 = 5_000_000;

            let untaxed: i32  = floor(f64::from(profit) * TAX_PERCENT / 100.0);
            let tax: i32 = FEE_CAP.min(untaxed);

            profit - tax
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn total_details_price(price_details: &[(i32, f32)]) -> i32 {
        // total price for each item is price * quantity
        let total: f32 = price_details.iter().map(|t| t.0 as f32 * t.1).sum();
        floor(f64::from(total))
    }

    // #[allow(clippy::cast_precision_loss)]
    // /// Calculates the total recipe time, parsing invalid times
    // pub fn recipe_time_h_option(
    //     recipe: &Recipe,
    //     number: i32,
    //     margin: i32,
    //     total_margin: bool,
    // ) -> (Option<f32>, Option<i32>) {
    //     if let RecipeTime::Time(t) = recipe.ticks {
    //         let (total_time_h, gp_h) = Self::recipe_time_h(t, number,
    //             margin, total_margin
    //         );
    //         return (Some(total_time_h), Some(gp_h));
    //     }
    //     (None, None)
    // }

    #[allow(clippy::cast_precision_loss)]
    pub fn recipe_time_h(
        time: f32,
        number: i32,
        margin: i32,
        total_margin: bool,
    ) -> (f32, i32) {
            let time_h: f32 = time / (60. * 60.);
            let total_time_h: f32 = f_round(number as f32 * time_h, 2);

            let gp_h = if total_margin {
                floor(f64::from(margin) / f64::from(total_time_h))
            } else {
                floor(f64::from(margin) / f64::from(time_h))
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
        // TODO: More efficient way of checking length?
        // E.g. early exiting from the iterator instead of filter_map?
        let filtered_items: Vec<_> = item_list
            .iter()
            .filter_map(|(item_name, &quantity)|
                // self.all_items.item_by_name(item_name) // str -> Item
                // .map(|item| (item.clone(), quantity)) // Item -> (Item, f32)
                {
                    let item: Option<&Item> = self.all_items.item_by_name(item_name);
                    item.map(|i| (i.clone(), quantity))
                }
            ).collect();

        if item_list.len() == filtered_items.len() {
            Some(filtered_items)
        } else {
            None
        }
    }

}
