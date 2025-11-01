use std::collections::HashSet;
use std::fs;

use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let path: PathBuf = PathBuf::from("src").join("bin").join("wiki_info");
    let new_path: PathBuf = path.join("wiki_allowed_recipes.txt");
    let old_path: PathBuf = PathBuf::from(new_path.to_str().unwrap().to_string() + ".bak");

    let new_file = fs::read_to_string(new_path)?;
    let old_file = fs::read_to_string(old_path)?;

    let old_recipes: HashSet<_> = old_file.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let new_recipes: HashSet<_> = new_file.lines().map(str::trim).filter(|l| !l.is_empty()).collect();

    let only_in_new: Vec<_> = new_recipes.difference(&old_recipes).collect();
    let only_in_old: Vec<_> = old_recipes.difference(&new_recipes).collect();
    println!("Recipes only in new file: {:#?}", only_in_new);
    println!();
    println!("Recipes only in old file: {:#?}", only_in_old);

    Ok(())
}
