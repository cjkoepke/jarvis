use super::ReviewModel;
use crate::paths::Env;
use serde::Deserialize;
use std::fs;

const CATALOG: &str = ".codex/models_cache.json";
const SHOWN_IN_PICKER: &str = "list";

#[derive(Deserialize)]
struct Catalog {
    models: Vec<CatalogModel>,
}

#[derive(Deserialize)]
struct CatalogModel {
    slug: String,
    description: Option<String>,
    visibility: Option<String>,
    priority: Option<i64>,
}

/// The models Codex offers in its own `/model` picker, in the same order.
pub fn offered(env: &Env) -> Vec<ReviewModel> {
    let catalog = fs::read_to_string(env.home.join(CATALOG)).unwrap_or_default();
    parse(&catalog).unwrap_or_default()
}

fn parse(catalog: &str) -> Option<Vec<ReviewModel>> {
    let catalog: Catalog = serde_json::from_str(catalog).ok()?;
    let mut models = catalog.models;
    models.retain(|model| model.visibility.as_deref() == Some(SHOWN_IN_PICKER));
    models.sort_by_key(|model| model.priority);
    let offered = models.into_iter().map(|model| ReviewModel {
        name: model.slug,
        about: model.description.unwrap_or_default(),
    });
    Some(offered.collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offers_the_listed_models_in_codex_order_and_none_from_a_catalog_it_cannot_read() {
        let catalog = r#"{"models": [
            {"slug": "second", "visibility": "list", "priority": 2, "extra": true},
            {"slug": "hidden", "visibility": "hide", "priority": 0},
            {"slug": "first", "description": "newest", "visibility": "list", "priority": 1}
        ]}"#;

        let names: Vec<String> = parse(catalog).unwrap().into_iter().map(|model| model.name).collect();

        assert_eq!(names, ["first", "second"]);
        assert!(parse("not json").is_none());
    }
}
