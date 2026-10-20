//! 非覆盖导入的关系重映射；不复用本机碰巧相同的来源 ID。
use super::{Cell, Data, invalid, strip_detection_token};
use crate::Result;
use std::collections::{HashMap, HashSet};

fn mapped(value: &mut Cell, mapping: &HashMap<String, String>) -> Result<()> {
    if let Some(old) = value.optional_text()? {
        value.set_text(mapping.get(old).ok_or_else(invalid)?.clone());
    }
    Ok(())
}
fn remap_layout(node: &mut crate::SavedLayout, profiles: &mut HashMap<String, String>) {
    match node {
        crate::SavedLayout::Connection { profile_id } => {
            // 已删除的来源引用保持悬空，禁止与本机恰巧相同的旧 ID 绑定。
            *profile_id = profiles
                .entry(profile_id.clone())
                .or_insert_with(|| uuid::Uuid::new_v4().to_string())
                .clone();
        }
        crate::SavedLayout::Split { first, second, .. } => {
            remap_layout(first, profiles);
            remap_layout(second, profiles);
        }
    }
}
pub(super) async fn remap(
    data: &mut Data,
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
) -> Result<()> {
    let mapping = |rows: &Vec<Vec<Cell>>| -> Result<HashMap<String, String>> {
        rows.iter()
            .map(|row| Ok((row[0].text()?.to_owned(), uuid::Uuid::new_v4().to_string())))
            .collect()
    };
    let groups = mapping(&data[0])?;
    let mut profiles = mapping(&data[1])?;
    let credentials = mapping(&data[2])?;
    let mut tags = mapping(&data[5])?;
    for row in &data[5] {
        if let Some(id) = sqlx::query_scalar::<_, String>("SELECT id FROM tags WHERE name=?")
            .bind(row[1].text()?)
            .fetch_optional(&mut **tx)
            .await?
        {
            tags.insert(row[0].text()?.to_owned(), id);
        }
    }
    let names:Vec<String>=sqlx::query_scalar("SELECT trim(name) FROM connection_profiles WHERE kind IN ('ssh','telnet','serial','workspace')").fetch_all(&mut **tx).await?;
    let mut names: HashSet<String> = names.into_iter().collect();
    for row in &mut data[0] {
        mapped(&mut row[0], &groups)?;
        mapped(&mut row[1], &groups)?;
        row[4] = Cell::Integer(1);
    }
    for row in &mut data[1] {
        mapped(&mut row[0], &profiles)?;
        mapped(&mut row[3], &groups)?;
        let base = row[2].text()?.to_owned();
        if !names.insert(base.clone()) {
            let mut number = 1;
            loop {
                let suffix = format!(" (同步 {number})");
                let candidate = format!(
                    "{}{}",
                    base.chars()
                        .take(128 - suffix.chars().count())
                        .collect::<String>(),
                    suffix
                );
                if names.insert(candidate.clone()) {
                    row[2].set_text(candidate);
                    break;
                }
                number += 1;
            }
        }
        if row[1].text()? == "workspace" {
            let mut config: serde_json::Value = serde_json::from_str(row[6].text()?)?;
            let mut workspace: crate::SavedWorkspace =
                serde_json::from_value(config["workspace"].clone())?;
            remap_layout(&mut workspace.root, &mut profiles);
            config["workspace"] = serde_json::to_value(workspace)?;
            row[6].set_text(serde_json::to_string(&config)?);
        }
        strip_detection_token(row)?;
        row[7] = Cell::Integer(1);
    }
    for row in &mut data[2] {
        mapped(&mut row[0], &credentials)?;
        mapped(&mut row[3], &profiles)?;
    }
    for row in &mut data[3] {
        mapped(&mut row[0], &profiles)?;
    }
    for row in &mut data[4] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
        mapped(&mut row[1], &profiles)?;
        mapped(&mut row[4], &credentials)?;
    }
    for row in &mut data[5] {
        mapped(&mut row[0], &tags)?;
    }
    for row in &mut data[6] {
        mapped(&mut row[0], &profiles)?;
        mapped(&mut row[1], &tags)?;
    }
    for row in &mut data[7] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
        mapped(&mut row[2], &profiles)?;
        row[4] = Cell::Integer(1);
    }
    for row in &mut data[8] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
    }
    for row in &mut data[9] {
        row[0].set_text(uuid::Uuid::new_v4().to_string());
        row[3] = Cell::Integer(1);
    }
    Ok(())
}
