use crate::{
    config::Membership,
    helpers::{f_round, floor},
    item_search::{
        item_search::{Item, ItemSearch},
        recipes::{Recipe, RecipeBook, RecipeTime},
    },
};

use std::collections::HashMap;

use super::pareto_sort::custom_types::{optimal_sort, Weights};

use tracing::{debug, warn};

use crate::helpers::number_to_comma_sep_string;

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

const SECOND_PER_TICK: f32 = 0.6;

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
        sort_by_weights: &Weights,
        price_options: &crate::config::Display,
    ) -> Table {
        // let [profiting, show_hidden, reverse] = price_options;
        let profiting = price_options.must_profit;
        let show_hidden = price_options.show_hidden;
        let reverse = price_options.reverse;
        let membership_option = &price_options.membership;

        let recipe_list = self.recipe_list.get_all_recipes();
        assert!(!recipe_list.is_empty());

        let all_recipe_prices = recipe_list
            .keys()
            .filter_map(|recipe_name| {
                let overview = self.recipe_price_overview_from_string(recipe_name)?;
                Some((recipe_name, overview))
            })
            .collect::<HashMap<_, _>>();
        assert!(!all_recipe_prices.is_empty());

        let mut all_recipe_details = Table::new();

        let coins = self.coins;
        for (recipe_name, overview) in all_recipe_prices {
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

            if !overview.3.isvalid() {
                warn!(desc = "INVALID RecipeTime", name = %recipe_name);
                continue;
            }

            let crate::item_search::recipes::RecipeTime::Time(time) = overview.3 else {
                unreachable!("INVALID RecipeTime should already be checked");
            };

            #[allow(clippy::cast_precision_loss)]
            // Times and GP/H will be low in comparison to max values
            let [recipe_cost_f, margin_f, time] =
                [overview.0 as f32, overview.2 as f32, time * SECOND_PER_TICK];

            let margin = floor(f64::from(margin_f));
            let recipe_cost = floor(f64::from(recipe_cost_f));

            let cant_afford = coins < recipe_cost;
            let no_profit = margin <= 0;

            // TODO: Assign some boolean values to variables so reused?
            // Or leave to the compiler instead?

            // Used Karnaugh map to calculate
            if (cant_afford && !show_hidden) || (no_profit && profiting && !show_hidden) {
                debug!(desc = "Skipping recipe from Karnaugh map...", name = %recipe_name);
                continue;
            }

            let [rn_s, m_s, totm_s, tt_s, gph_s] = if (cant_afford && show_hidden)
                || (no_profit && profiting && show_hidden)
            {
                [
                    recipe_name.to_owned(),
                    "#".to_owned(),
                    "#".to_owned(),
                    "#".to_owned(),
                    "#".to_owned(),
                ]
            } else {
                let amount = floor(f64::from(coins) / f64::from(recipe_cost));

                let (total_time_h, gp_h) = PriceHandle::recipe_time_h(time, amount, margin, false);
                [
                    recipe_name.to_owned(),
                    number_to_comma_sep_string(&margin),
                    number_to_comma_sep_string(&(amount * margin)),
                    total_time_h.to_string(),
                    number_to_comma_sep_string(&gp_h),
                ]
            };

            let row: _TableRow = [rn_s, m_s, totm_s, tt_s, gph_s];

            all_recipe_details.push(row);
        }

        // all_recipe_details
        // TODO: Does this actually modify?
        optimal_sort(&all_recipe_details, sort_by_weights, reverse)
    }

    pub fn recipe_lookup_from_recipe(&self, recipe: &Recipe) -> Option<Vec<Vec<String>>> {
        // TODO: Change types to Row/Table?
        let mut recipe_lookup: Vec<Vec<String>> = Vec::new();

        // Need to parse item strings into Item objects
        let input_items = self.parse_item_list(&recipe.inputs)?;
        let output_items = self.parse_item_list(&recipe.outputs)?;

        // HashMap[item -> (price, quantity)]
        // Base price
        let input_details = PriceHandle::item_list_prices_unchecked(input_items, true);
        let output_details = PriceHandle::item_list_prices_unchecked(output_items, false);

        // Base price
        let cost = PriceHandle::total_details_price(
            &input_details.clone().into_values().collect::<Vec<_>>(),
        );
        let revenue_untaxed = PriceHandle::total_details_price(
            &output_details.clone().into_values().collect::<Vec<_>>(),
        );
        let revenue_taxed = PriceHandle::apply_tax(revenue_untaxed);

        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        // Adjust prices according to price_margin
        // Increase buy price, descrease sell price
        // Note plus and minus symbols differ
        let input_details_pm = input_details
            .clone()
            .into_iter()
            .map(|(item, (price, quantity))| {
                let adj_price = ((price as f32) * (1. + self.pmargin / 100.)).floor();
                (item, (adj_price as i32, quantity))
            })
            .collect::<HashMap<Item, (i32, f32)>>();

        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        let output_details_pm = output_details
            .clone()
            .into_iter()
            .map(|(item, (price, quantity))| {
                let adj_price = ((price as f32) * (1. - self.pmargin / 100.)).floor();
                (item, (adj_price as i32, quantity))
            })
            .collect::<HashMap<Item, (i32, f32)>>();

        let profit = revenue_taxed - cost;

        // Percent margin adjusted
        let cost_pm = PriceHandle::total_details_price(
            &input_details_pm.clone().into_values().collect::<Vec<_>>(),
        );
        let revenue_untaxed_pm = PriceHandle::total_details_price(
            &output_details_pm.clone().into_values().collect::<Vec<_>>(),
        );
        let revenue_taxed_pm = PriceHandle::apply_tax(revenue_untaxed_pm);

        let profit_pm = revenue_taxed_pm - cost_pm;

        // Calculate number of recipes that can be made
        let number: i32 = self.coins / cost;
        let number_pm: i32 = self.coins / cost_pm;

        // Calculate total potential profit margin if all money is used
        let total_profit: i32 = profit * number;
        let total_profit_pm: i32 = profit_pm * number_pm;

        let time = recipe.ticks.clone();

        // Already have recipe time so can skip lookup
        // First variable is time string for single recipe
        let (_, (tt_s, gph_s), (tt_pm_s, gph_pm_s)) = match time {
            RecipeTime::INVALID => (
                String::new(),
                (String::new(), String::new()),
                (String::new(), String::new()),
            ),
            RecipeTime::Time(recipe_time) => {
                let recipe_time = recipe_time * SECOND_PER_TICK;

                // Regular and profit margin adjusted times & gph
                let norm = PriceHandle::recipe_time_h(recipe_time, number, total_profit, true);
                let pm = PriceHandle::recipe_time_h(recipe_time, number_pm, total_profit_pm, true);

                (
                    recipe_time.to_string(),
                    (norm.0.to_string(), number_to_comma_sep_string(&norm.1)),
                    (pm.0.to_string(), number_to_comma_sep_string(&pm.1)),
                )
            }
        };

        // Form table

        // Header row
        recipe_lookup.insert(
            0,
            vec![
                "Item".to_owned(),
                "Amount".to_owned(),
                "To Buy".to_owned(),
                "Price".to_owned(),
                "Total Price".to_owned(),
                "Total Time (h)".to_owned(),
                "Profit/Recipe Time (GP/h)".to_owned(),
            ],
        );

        recipe_lookup.push(vec!["Inputs (Base)".to_owned()]);

        // Iterate through items and add rows
        // item_details is in the form [item -> (price, quantity (amount for single recipe)]
        // Can multiply by number to get (to buy) and (total price) for each item
        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        for (item, (price, amount)) in input_details {
            let to_buy: i32 = (amount * (number as f32)).floor() as i32;
            let total_item_price = price * number;

            let row: Vec<String> = vec![
                item.name.clone(),
                number_to_comma_sep_string(&(amount as i32)),
                number_to_comma_sep_string(&to_buy),
                number_to_comma_sep_string(&price),
                number_to_comma_sep_string(&total_item_price),
            ];

            recipe_lookup.push(row);
        }

        recipe_lookup.push(vec![
            "Total (Base)".to_owned(),
            String::new(),
            String::new(),
            number_to_comma_sep_string(&cost),
            number_to_comma_sep_string(&(cost * number)),
        ]);

        recipe_lookup.push(Vec::new()); // Empty row

        recipe_lookup.push(vec!["Outputs (Base)".to_owned()]);

        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        for (item, (price, amount)) in output_details {
            let to_buy: i32 = (amount * (number as f32)).floor() as i32;
            let total_item_price = price * number;

            let row: Vec<String> = vec![
                item.name.clone(),
                number_to_comma_sep_string(&(amount as i32)),
                number_to_comma_sep_string(&to_buy),
                number_to_comma_sep_string(&price),
                number_to_comma_sep_string(&total_item_price),
            ];

            recipe_lookup.push(row);
        }

        recipe_lookup.push(vec![
            "Total (w/Tax Base)".to_owned(),
            String::new(),
            String::new(),
            number_to_comma_sep_string(&revenue_taxed),
            number_to_comma_sep_string(&(revenue_taxed * number)),
        ]);

        recipe_lookup.push(Vec::new());

        recipe_lookup.push(vec![
            "Profit/Loss (w/Tax Base)".to_owned(),
            number_to_comma_sep_string(&number),
            String::new(),
            number_to_comma_sep_string(&profit),
            number_to_comma_sep_string(&total_profit),
            tt_s,
            gph_s,
        ]);

        recipe_lookup.push(Vec::new());
        recipe_lookup.push(Vec::new());

        // Now repeat with percent margin values
        recipe_lookup.push(vec![
            // &"Inputs (2.50% margin)",
            format!("Inputs ({}% margin)", self.pmargin),
        ]);

        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        for (item, (price, amount)) in input_details_pm {
            let to_buy: i32 = (amount * (number_pm as f32)).floor() as i32;
            let total_item_price = price * number_pm;

            let row: Vec<String> = vec![
                item.name.clone(),
                number_to_comma_sep_string(&(amount as i32)),
                number_to_comma_sep_string(&to_buy),
                number_to_comma_sep_string(&price),
                number_to_comma_sep_string(&total_item_price),
            ];

            recipe_lookup.push(row);
        }

        recipe_lookup.push(vec![
            format!("Total ({}% margin)", self.pmargin),
            String::new(),
            String::new(),
            number_to_comma_sep_string(&cost_pm),
            number_to_comma_sep_string(&(cost_pm * number_pm)),
        ]);

        recipe_lookup.push(Vec::new());

        recipe_lookup.push(vec![format!("Outputs ({}% margin)", self.pmargin)]);

        #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
        for (item, (price, amount)) in output_details_pm {
            let to_buy: i32 = (amount * (number_pm as f32)).floor() as i32;
            let total_item_price = price * number_pm;

            let row: Vec<String> = vec![
                item.name.clone(),
                number_to_comma_sep_string(&(amount as i32)),
                number_to_comma_sep_string(&to_buy),
                number_to_comma_sep_string(&price),
                number_to_comma_sep_string(&total_item_price),
            ];

            recipe_lookup.push(row);
        }

        recipe_lookup.push(vec![
            format!("Total (w/Tax {}% margin)", self.pmargin),
            String::new(),
            String::new(),
            number_to_comma_sep_string(&revenue_taxed_pm),
            number_to_comma_sep_string(&(revenue_taxed_pm * number)),
        ]);

        recipe_lookup.push(Vec::new());

        recipe_lookup.push(vec![
            format!("Profit/Loss (w/Tax {}% margin)", self.pmargin),
            number_to_comma_sep_string(&number_pm),
            String::new(),
            number_to_comma_sep_string(&profit_pm),
            number_to_comma_sep_string(&total_profit_pm),
            tt_pm_s,
            gph_pm_s,
        ]);

        Some(recipe_lookup)
    }

    pub fn recipe_price_overview_from_string(&self, recipe_name: &String) -> Option<Row> {
        let recipe = self.recipe_list.get_recipe(recipe_name)?;
        self.recipe_price_overview_from_recipe(recipe)
    }

    #[allow(clippy::missing_panics_doc, reason = "infallible")]
    pub fn recipe_price_overview_from_recipe(&self, recipe: &Recipe) -> Option<Row> {
        // Need to parse item strings into Item objects
        let input_items = self.parse_item_list(&recipe.inputs)?;
        let output_items = self.parse_item_list(&recipe.outputs)?;

        let input_details = PriceHandle::item_list_prices_unchecked(input_items, true);
        assert!(!input_details.is_empty());

        let output_details = PriceHandle::item_list_prices_unchecked(output_items, false);
        assert!(!output_details.is_empty());

        let cost =
            PriceHandle::total_details_price(&input_details.into_values().collect::<Vec<_>>());

        let revenue = PriceHandle::apply_tax(PriceHandle::total_details_price(
            &output_details.into_values().collect::<Vec<_>>(),
        ));

        let profit = revenue - cost;

        let time = recipe.ticks.clone();

        Some((cost, revenue, profit, time))
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

            let untaxed: i32 = floor(f64::from(profit) * TAX_PERCENT / 100.0);
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
    /// Returns total time in hours, and estimated GP/hour
    pub fn recipe_time_h(time: f32, number: i32, margin: i32, total_margin: bool) -> (f32, i32) {
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
    /// true means buy, false means sell
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
        let filtered_items: Vec<Option<_>> = item_list
            .iter()
            .map(|(item_name, &quantity)| {
                self.all_items
                    .item_by_name(item_name)
                    .map(|item_option| (item_option.clone(), quantity))
            })
            .take_while(Option::is_some)
            .collect();

        if item_list.len() == filtered_items.len() {
            // SAFETY: Know all elements are in lookup and are type Item
            Some(filtered_items.into_iter().map(Option::unwrap).collect())
        } else {
            None
        }
    }
}
