use anyhow::{Context, Result};
use serde_yaml::{Mapping as YamlMapping, Sequence as YamlSequence, Value as YamlValue};

use crate::gen::doc2::DocsItemTag;

use super::doc2::{DocItem, DocItemKind, DocTree};

pub fn generate_from_type_definitions(contents: &str) -> Result<String> {
    let tree = DocTree::from_type_definitions(contents)?;
    let mut globals = YamlMapping::new();
    let top_level_items = tree.children().iter().filter(|top_level| {
        top_level.is_function()
            || top_level.children().iter().any(|top_level_child| {
                top_level_child.is_tag() && top_level_child.get_name().unwrap() == "class"
            })
    });
    for top_level_item in top_level_items {
        match top_level_item.kind() {
            DocItemKind::Table => {
                let top_level_name = top_level_item
                    .get_name()
                    .context("Missing name for top-level doc item")?
                    .to_string();
                for child_item in top_level_item
                    .children()
                    .iter()
                    .filter(|item| item.is_function() || item.is_property())
                {
                    let child_name = child_item
                        .get_name()
                        .context("Missing name for top-level child doc item")?
                        .to_string();
                    globals.insert(
                        YamlValue::String(format!("{top_level_name}.{child_name}")),
                        YamlValue::Mapping(doc_item_to_selene_yaml_mapping(child_item)?),
                    );
                }
            }
            DocItemKind::Function => {
                globals.insert(
                    YamlValue::String(
                        top_level_item
                            .get_name()
                            .context("Missing name for top-level doc item")?
                            .to_string(),
                    ),
                    YamlValue::Mapping(doc_item_to_selene_yaml_mapping(top_level_item)?),
                );
            }
            _ => unimplemented!("Globals other than tables and functions are not yet implemented"),
        }
    }
    let mut contents = YamlMapping::new();
    contents.insert(
        YamlValue::String("globals".to_string()),
        YamlValue::Mapping(globals),
    );
    Ok(format!(
        "# Lune v{}\n---\n{}",
        env!("CARGO_PKG_VERSION"),
        serde_yaml::to_string(&contents).context("Failed to encode type definitions as yaml")?
    ))
}

fn doc_item_to_selene_yaml_mapping(item: &DocItem) -> Result<YamlMapping> {
    let mut mapping = YamlMapping::new();
    if item.is_property() {
        let property_access_tag = item
            .children()
            .iter()
            .find_map(|child| {
                if let Ok(tag) = DocsItemTag::try_from(child) {
                    if tag.is_read_only() || tag.is_read_write() {
                        Some(tag)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .with_context(|| {
                format!(
                    "Missing property access tag for doc item:\n{}",
                    item.get_name().unwrap()
                )
            })?;
        mapping.insert(
            YamlValue::String("property".to_string()),
            YamlValue::String(
                match property_access_tag {
                    DocsItemTag::ReadOnly => "read-only",
                    DocsItemTag::ReadWrite => "new-fields",
                    _ => unreachable!(),
                }
                .to_string(),
            ),
        );
    } else if item.is_function() {
        let is_must_use = item.children().iter().any(|child| {
            if let Ok(tag) = DocsItemTag::try_from(child) {
                tag.is_must_use()
            } else {
                false
            }
        });
        if is_must_use {
            mapping.insert(
                YamlValue::String("must_use".to_string()),
                YamlValue::Bool(true),
            );
        }
        mapping.insert(
            YamlValue::String("args".to_string()),
            YamlValue::Mapping(YamlMapping::new()),
        );
    }
    Ok(mapping)
}
