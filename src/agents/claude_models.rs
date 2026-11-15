use super::ReviewModel;
use crate::files::sorted_entries;
use crate::paths::Env;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const CATALOG_DIR: &str = ".claude/cache/model-catalog";
const CATALOG_SUFFIX: &str = "-cc.json";
const SHOWN_IN_PICKER: &str = "main";

#[derive(Deserialize)]
struct CatalogFile {
    catalog: Catalog,
}

#[derive(Deserialize)]
struct Catalog {
    config: CatalogConfig,
}

#[derive(Deserialize)]
struct CatalogConfig {
    models: Vec<CatalogModel>,
}

#[derive(Deserialize)]
struct CatalogModel {
    id: String,
    description: Option<String>,
    section: Option<String>,
}

/// The models Claude Code offers in its own `/model` picker, in the same order.
pub fn offered(env: &Env) -> Vec<ReviewModel> {
    let catalog = newest_catalog(env)
        .and_then(|file| fs::read_to_string(file).ok())
        .unwrap_or_default();
    parse(&catalog).unwrap_or_default()
}

fn newest_catalog(env: &Env) -> Option<PathBuf> {
    let catalogs = sorted_entries(&env.home.join(CATALOG_DIR)).ok()?;
    let is_claude_code_catalog = |file: &PathBuf| file.to_string_lossy().ends_with(CATALOG_SUFFIX);
    let modified = |file: &PathBuf| fs::metadata(file).and_then(|metadata| metadata.modified()).ok();
    catalogs.into_iter().filter(is_claude_code_catalog).max_by_key(modified)
}

fn parse(catalog: &str) -> Option<Vec<ReviewModel>> {
    let file: CatalogFile = serde_json::from_str(catalog).ok()?;
    let mut models = file.catalog.config.models;
    models.retain(|model| model.section.as_deref() == Some(SHOWN_IN_PICKER));
    let offered = models.into_iter().map(|model| ReviewModel {
        name: model.id,
        about: model.description.unwrap_or_default(),
    });
    Some(offered.collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_the_main_models_in_claude_code_order_and_none_from_a_catalog_it_cannot_read() {
        let catalog = r#"{"catalog": {"config": {"models": [
            {"id": "first", "description": "newest", "section": "main", "extra": true},
            {"id": "older", "description": null, "section": "overflow"},
            {"id": "second", "section": "main"}
        ]}}}"#;

        let names: Vec<String> = parse(catalog).unwrap().into_iter().map(|model| model.name).collect();

        assert_eq!(names, ["first", "second"]);
        assert!(parse("not json").is_none());
    }
}
