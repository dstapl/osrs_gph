//! Pareto sort implementation

type Weights = [f32; 4];

// pub mod actual_types {
// use std::cmp::Ordering;
//
// use itertools::Itertools;
//     use prettytable::{Row, Table};
//     #[allow(clippy::cast_precision_loss)]
//     fn lin_scalarization(x: &Row, weights: &Weights) -> Option<f32> {
//         // Want to multiply weights with x, starting from 2nd element (i32)
//         // Need to skip the first element of the Row (String)
//         let parsed = x.iter()
//             .skip(1)
//             .filter_map(|c| {
//                 let cont = c.get_content();
//                 if cont.contains(',') {
//                     let p = parse_comma_string(&cont);
//                     // dbg!(&p);
//                     p.ok().map(|a| a as f32)
//                 } else {
//                     cont.parse::<f32>().ok()
//
//                 }
//             })
//             .collect_vec();
//         // dbg!(&parsed);
//         if parsed.len() == 4 {
//             Some(
//                 parsed.iter().zip(weights.iter())
//                 .map(|(a, b)| a * b)
//                 .sum()
//             )
//         } else {
//             None
//         }
//
//     }
//
//     fn ls_compare(x: &Row, y: &Row, weights: &Weights) -> Ordering {
//         let Some(x_val) = lin_scalarization(x, weights)
//             else { return Ordering::Less };
//         let Some(y_val) = lin_scalarization(y, weights)
//             else { return Ordering::Greater };
//
//         // dbg!(( (x_val, y_val) ));
//         // (x_val > y_val) - (x_val < y_val)
//         // Want x_val > y_val (Opposite of x_val < y_val)
//         x_val.total_cmp(&y_val)
//     }
//     #[must_use]
//     /// Modifying function
//     pub fn optimal_sort(table: &Table, weights: &Weights, reverse: bool) -> Table{
//         // let norm_weights = normalize_weights(weights); // Normalize
//                                                    // Return sorted based on ls_compare function
//         let norm_weights = *weights;
//         let row_list = if reverse {
//             table.row_iter().sorted_by(|a, b| ls_compare(b, a, &norm_weights)) // Might need to change to cmp::Reverse
//         } else {
//             table.row_iter().sorted_by(|a, b| ls_compare(a, b, &norm_weights))
//         };
//
//         let mut output_table = Table::new();
//         for row in row_list {
//             output_table.add_row(row.clone());
//         }
//         output_table
//
//     }
//
//     #[must_use]
//     pub fn compute_weights(coins: i32, config_weights: [f32; 3]) -> Weights {
//         let [margin, time, gp_h] = config_weights;
//
//         #[allow(clippy::cast_precision_loss)]
//         let denom = 10_f32.powf((coins as f32).log10() - 1.);
//
//         let money_to_time = (margin, 1./denom);
//         let ratio = 1./(money_to_time.0 + money_to_time.1);
//
//         [money_to_time.0/ratio, money_to_time.1/ratio, time, gp_h]
//     }
//
//     #[must_use]
//     /// Normalise weights
//     pub fn normalize_weights(weights: &Weights) -> Weights {
//         let w_sum: f32 = weights.iter().sum();
//         let mut norm_weights = *weights;
//         let _ = norm_weights.iter_mut().map(|w|
//             *w /= w_sum
//         );
//         norm_weights
//     }
// }

pub mod custom_types {
    use std::cmp::Ordering;

    use itertools::Itertools;

    use crate::types::OverviewRow;

    pub type Weights = super::Weights;

    fn lin_scalarization(x: &OverviewRow, weights: &Weights) -> f32 {
        #[allow(clippy::cast_precision_loss)]
        [
            x.profit as f32,
            x.total_gp() as f32,
            x.total_time(),
            x.gph() as f32,
        ]
        .iter()
        .zip(weights.iter())
        .map(|(a, b)| a * b)
        .sum()
    }

    fn ls_compare(x: &OverviewRow, y: &OverviewRow, weights: &Weights) -> Ordering {
        let x_val = lin_scalarization(x, weights);
        let y_val = lin_scalarization(y, weights);

        // dbg!(( (x_val, y_val) ));
        // (x_val > y_val) - (x_val < y_val)
        // Want x_val > y_val (Opposite of x_val < y_val)
        x_val.total_cmp(&y_val)
    }

    pub fn optimal_sort(
        table: &[OverviewRow],
        weights: &Weights,
        reverse: bool,
    ) -> Vec<OverviewRow> {
        // let norm_weights = normalize_weights(weights); // Normalize
        // Return sorted based on ls_compare function
        // let norm_weights = *weights;
        let norm_weights = weights;
        let row_list = if reverse {
            table
                .iter()
                .sorted_by(|a, b| ls_compare(b, a, norm_weights)) // Might need to change to cmp::Reverse
        } else {
            table
                .iter()
                .sorted_by(|a, b| ls_compare(a, b, norm_weights))
        };

        let mut output_table = Vec::new();
        for row in row_list {
            output_table.push(row.clone());
        }
        output_table
    }

    // TODO: What is the reasoning for this?
    pub fn compute_weights(coins: i32, config_weights: &crate::config::Weights) -> Weights {
        // let [margin, time, gp_h] = config_weights;
        let margin = config_weights.margin;
        let time = config_weights.time;
        let gp_h = config_weights.gph;

        #[allow(clippy::cast_precision_loss)]
        let money_to_time = (margin, 10.0 / (coins as f32));
        let factor = money_to_time.0 + money_to_time.1;

        [
            money_to_time.0 * factor,
            money_to_time.1 * factor,
            time,
            gp_h,
        ]
    }

    /// Normalise weights
    pub fn normalize_weights(weights: &Weights) -> Weights {
        let w_sum: f32 = weights.iter().sum();
        let mut norm_weights = *weights;
        let _ = norm_weights.iter_mut().map(|w| *w /= w_sum);
        norm_weights
    }
}
