use crate::{
    config::{Membership, OverviewFilter, OverviewSortBy},
    helpers::f_round,
    item_search::{
        item_search::{Item, ItemSearch},
        recipes::{Recipe, RecipeBook, RecipeTime},
    },
    types::{DetailedTable, OverviewRow, SEC_IN_HOUR},
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

        let all_recipe_prices = recipe_list
            .keys()
            .filter_map(|recipe_name| {
                let overview_output = self.recipe_price_overview_from_string(recipe_name)?;
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

            let profit = overview.profit;
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
                };
            },
            // TODO: Are these the same order as total times / total profit?
            OverviewSortBy::Profit => {
                all_overviews.sort_by_key(|k| k.total_gp());
                if !reverse { // Highest profit first
                    all_overviews.reverse();
                };
            },
            OverviewSortBy::Time => {
                all_overviews.sort_by_key(|k| (k.total_time() * SEC_IN_HOUR as f32) as i32);
                if reverse {
                    all_overviews.reverse();
                };
            },
            OverviewSortBy::GPH => {
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

    pub fn recipe_lookup_from_recipe(&self, recipe: &Recipe) -> Option<DetailedTable> {
        // Need to parse item strings into Item objects
        let input_items = self.parse_item_list(&recipe.inputs)?;
        let output_items = self.parse_item_list(&recipe.outputs)?;

        // HashMap[item -> (price, quantity)]
        // Base price
        let input_details = PriceHandle::item_list_prices_unchecked(input_items, true);
        let output_details = PriceHandle::item_list_prices_unchecked(output_items, false);

        let (overview, (_,_)) = self.recipe_price_overview_from_recipe(recipe)?;

        // Form table
        // Transform input/outputs to DetailedTable type
        // TODO: Switch DetailedTable input/outputs types to HashMap instead
        let input_vec = input_details.into_iter()
            .map(|(item, (price, quantity))| (item.name,price,quantity))
            .collect();
        let output_vec = output_details.into_iter()
            .map(|(item, (price, quantity))| (item.name,price,quantity))
            .collect();

        let recipe_lookup: DetailedTable = DetailedTable::new(
            overview,
            input_vec,
            output_vec,
            self.pmargin,
        );

        Some(recipe_lookup)
    }

    pub fn recipe_price_overview_from_string(&self, recipe_name: &String) -> Option<(OverviewRow, (i32, i32))>  {
        let recipe = self.recipe_list.get_recipe(recipe_name)?;
        self.recipe_price_overview_from_recipe(recipe)
    }

    /// Returns price overview and cost of inputs and (taxed) revenue from outputs
    #[allow(clippy::missing_panics_doc, reason = "infallible")]
    pub fn recipe_price_overview_from_recipe(&self, recipe: &Recipe) -> Option<(OverviewRow, (i32, i32))> {
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

        let time_ticks = recipe.ticks.clone();
        // Return None from function if Invalid
        let time_sec = match time_ticks {
            RecipeTime::INVALID => None,
            RecipeTime::Time(recipe_time) => Some(recipe_time * SECOND_PER_TICK)
        };

        if time_sec.is_none() {
                warn!(desc = "INVALID RecipeTime", name = %recipe.name);
                return None
        }

        let Some(time_sec) = time_sec else {
                unreachable!("INVALID RecipeTime should already be checked");
        };

        let number = self.coins / cost;

        let overview = OverviewRow::new(
            recipe.name.clone(),
            profit,
            time_sec,
            number
        );

        Some((overview, (cost, revenue)))
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

            #[allow(clippy::cast_possible_truncation)]
            // SAFETY: original values and multiplication are within i32 limit
            let untaxed = (f64::from(profit) * TAX_PERCENT / 100.0).floor() as i32;
            let tax: i32 = FEE_CAP.min(untaxed);

            profit - tax
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn total_details_price(price_details: &[(i32, f32)]) -> i32 {
        // total price for each item is price * quantity
        let total: f32 = price_details.iter().map(|t| t.0 as f32 * t.1).sum();

        #[allow(clippy::cast_possible_truncation)]
        // TODO: Max size of i32 < f32
        return total.floor() as i32
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
