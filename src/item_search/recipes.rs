//! Handling recipes defined in `lookup_data/recipes.yaml`
use serde::{de::Visitor, Deserialize};

use crate::{
    file_io::{FileIO, FileOptions},
    log_match_panic,
};
use tracing::{debug, trace, warn};

use std::{collections::HashMap, fmt::Debug};

// #[serde(untagged)]
#[derive(Debug, Default, Clone)]
pub enum RecipeTime {
    /// Time in ticks
    Time(f32),
    #[default]
    INVALID,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Recipe {
    pub name: String,
    pub members: bool,

    pub inputs: HashMap<String, f32>,
    pub outputs: HashMap<String, f32>,

    #[serde(alias = "time")]
    pub ticks: RecipeTime,
}

#[derive(Debug, Deserialize, Default)]
pub struct RecipeBook {
    pub recipes: HashMap<String, Recipe>,
}

impl<'de> Deserialize<'de> for RecipeTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RecipeTimeVisitor;
        impl Visitor<'_> for RecipeTimeVisitor {
            type Value = RecipeTime;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("f32 or no field")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RecipeTime::INVALID)
            }

            #[allow(clippy::cast_possible_truncation)]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok((v as f32).into())
            }
            #[allow(clippy::cast_precision_loss)]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok((v as f32).into())
            }
            #[allow(clippy::cast_precision_loss)]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok((v as f32).into())
            }
        }

        let de = deserializer.deserialize_f32(RecipeTimeVisitor);
        if let Err(e) = de {
            if e.to_string().contains("missing field") {
                Ok(RecipeTime::INVALID)
            } else {
                Err(e)
            }
        } else {
            de
        }
    }
}

impl Recipe {
    pub fn new<S: Into<String>, T: Into<RecipeTime>>(
        name: S,
        inputs: HashMap<String, f32>,
        outputs: HashMap<String, f32>,
        ticks: T,
    ) -> Self {
        Self {
            name: name.into(),
            members: false,
            inputs,
            outputs,
            ticks: ticks.into(),
        }
    }

    pub fn isvalid(&self) -> bool {
        self.ticks.isvalid()
    }
}

impl RecipeBook {
    pub fn new<H: Into<HashMap<String, Recipe>>>(recipes: H) -> Self {
        Self {
            recipes: recipes.into(),
        }
    }

    pub fn load_default_recipes(&mut self, recipe_path: String) {
        // let recipes_fio = Logging::<FileIO<S>>::new(, recipe_path);
        let mut recipes_fio = FileIO::new(recipe_path, FileOptions::new(true, false, false));

        // TODO: Implement choice of other SerChoice options
        let file_output =
            recipes_fio.read_serialized::<HashMap<String, Recipe>>(crate::file_io::SerChoice::YAML);

        let mut recipe_list: Vec<Recipe> = log_match_panic(
            file_output,
            "Read recipe list from file.",
            "Failed to load recipes.",
        )
        .into_values()
        .collect();

        // Filer out invalid recipes; using .isvalid()
        // Log any invalid recipes
        let before_len = recipe_list.len();

        recipe_list.retain(|r| {
            if r.isvalid() {
                true
            } else {
                warn!(desc = "Skipping recipe.", recipe_name = %r.name);
                false
            }
        });

        debug!(
            "Filtered out {} invalid recipes.",
            before_len - recipe_list.len()
        );

        self.add_from_list(recipe_list);

        trace!("Restore old recipes in recipe_list");

        self.remove_recipe("Template");
        trace!("Remove Template recipe from recipe_list");

        debug!("Loaded {} recipes.", self.len());
    }

    pub fn add_recipe(&mut self, recipe: Recipe) -> Option<Recipe> {
        let recipe_name = recipe.name.clone();
        let recipe_o: Option<Recipe> = self.recipes.insert(recipe_name.clone(), recipe);

        if recipe_o.is_some() {
            trace!("Recipe `{}` already existed...updating", recipe_name);
        }

        recipe_o
    }

    pub fn add_from_list(&mut self, recipe_list: Vec<Recipe>) {
        // TODO: Is it faster to iterate twice(?)
        //  Once to get names from each recipe
        //  Another to join the names and recipes as (K,V)
        //  Final(?) to call self.recipes.extend(...)
        // Add in new recipes
        for recipe in recipe_list {
            self.add_recipe(recipe);
        }
    }

    pub fn remove_recipe<S: Into<String>>(&mut self, recipe_name: S) -> Option<Recipe> {
        self.recipes.remove(&recipe_name.into())
    }

    pub fn get_recipe(&self, recipe_name: &String) -> Option<&Recipe> {
        let rec = self.recipes.get(recipe_name);

        if rec.is_none() {
            warn!("Invalid recipe name: {}", recipe_name);
        }

        rec
    }

    pub fn get_all_recipes(&self) -> HashMap<String, Recipe> {
        self.recipes.clone()
    }

    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

impl From<HashMap<String, Recipe>> for RecipeBook {
    fn from(recipes: HashMap<String, Recipe>) -> Self {
        Self { recipes }
    }
}

impl RecipeTime {
    pub fn isvalid(&self) -> bool {
        !matches!(self, Self::INVALID)
    }
}

impl std::fmt::Display for RecipeTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecipeTime::Time(t) => write!(f, "{t}"),
            RecipeTime::INVALID => write!(f, ""),
        }
    }
}

impl<F: Into<f32>> From<F> for RecipeTime {
    fn from(value: F) -> Self {
        let f: f32 = value.into();
        if f < 0. {
            RecipeTime::INVALID
        } else {
            RecipeTime::Time(f)
        }
    }
}
