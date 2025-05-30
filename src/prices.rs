use crate::{
    helpers::{f_round, floor},
    item_search::{
        item_search::{Item, ItemSearch},
        recipes::{RecipeBook, RecipeTime, Recipe},
    }
};

use std::{collections::HashMap, path::Path, fmt::Display};

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
        let output_details = PriceHandle::item_list_prices_unchecked(
            output_items, false
        );

        let cost = PriceHandle::total_price(
            &input_details.into_values().collect::<Vec<_>>()
        );
        let revenue = PriceHandle::apply_tax(
            PriceHandle::total_price(
                &output_details.into_values().collect::<Vec<_>>()
            )
        );
        let profit = revenue-cost;
        let time = &recipe.ticks;

        Some(
            row![
            cost,
            revenue,
            profit,
            time
            ]
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
    pub fn total_price(price_details: &[(i32, f32)]) -> i32 {
        // total price for each item is price * quantity
        let total: f32 = price_details.iter().map(|t| t.0 as f32 * t.1).sum();
        floor(f64::from(total))
    }

    #[allow(clippy::cast_precision_loss)]
    /// Calculates the total recipe time, parsing invalid times
    pub fn recipe_time_h(
        recipe: &Recipe,
        number: i32,
        margin: i32,
        total_margin: bool,
    ) -> (Option<f32>, Option<i32>) {
        if let RecipeTime::Time(t) = recipe.ticks {
            let time_h: f32 = t / (60. * 60.);
            let total_time_h: f32 = f_round(number as f32 * time_h, 2);

            let gp_h = if total_margin {
                floor(f64::from(margin) / f64::from(total_time_h))
            } else {
                floor(f64::from(margin)/ f64::from(time_h))
            };

            return (Some(total_time_h), Some(gp_h));
        }
        (None, None)
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn recipe_time_h_manual(
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

    pub fn parse_item_list(&self, items: &HashMap<String, f32>) -> Option<Vec<(Item, f32)>> {
        let filtered_items: Vec<_> = items
            .iter()
            .filter_map(|(item_name, &quantity)|
                self.all_items
                .item_by_name(item_name)
                .map(|item| (item.clone(), quantity))
            ).collect();
        if items.len() == filtered_items.len() {
            Some(filtered_items)
        } else {
            None
        }
    }

}
