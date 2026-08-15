use crate::services::settings::SettingsService;
use std::fs;
use std::process::Command;

/// Namespace-to-module mapping for Niri layer-rules
const MODULE_NAMESPACES: &[(&str, &str)] = &[
    ("bar", "cos-bar"),
    ("quick_settings", "cos-quick-settings"),
    ("calendar", "cos-calendar"),
    ("launcher", "cos-launcher"),
    ("tray", "cos-tray-menu"),
];

pub struct NiriConfigService;

impl NiriConfigService {
    fn rules_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config/niri/rules.kdl")
    }

    /// Toggle blur for a specific module, patch rules.kdl, reload Niri
    pub fn set_blur(module: &str, enabled: bool) {
        let namespace = match MODULE_NAMESPACES.iter().find(|(m, _)| *m == module) {
            Some((_, ns)) => *ns,
            None => return,
        };

        SettingsService::set_blur(module, enabled);

        if let Err(e) = Self::patch_layer_rule(namespace, "blur", enabled) {
            eprintln!("[niri_config] Failed to patch blur for {}: {}", namespace, e);
            return;
        }

        Self::reload_niri();
    }

    /// Toggle xray for a specific module, patch rules.kdl, reload Niri
    pub fn set_xray(module: &str, enabled: bool) {
        let namespace = match MODULE_NAMESPACES.iter().find(|(m, _)| *m == module) {
            Some((_, ns)) => *ns,
            None => return,
        };

        SettingsService::set_xray(module, enabled);

        if let Err(e) = Self::patch_layer_rule(namespace, "xray", enabled) {
            eprintln!("[niri_config] Failed to patch xray for {}: {}", namespace, e);
            return;
        }

        Self::reload_niri();
    }

    /// Reset all module layer-rules to default (blur: true, xray: false)
    pub fn reset_rules() {
        for (_, ns) in MODULE_NAMESPACES {
            let _ = Self::patch_layer_rule(ns, "blur", true);
            let _ = Self::patch_layer_rule(ns, "xray", false);
        }
        Self::reload_niri();
    }

    /// Patch a specific property inside a layer-rule block matching a namespace.
    ///
    /// Strategy: Find the layer-rule block containing `match namespace="<ns>"`,
    /// then find/update or insert the property within its `background-effect` sub-block.
    fn patch_layer_rule(namespace: &str, property: &str, value: bool) -> Result<(), String> {
        let path = Self::rules_path();
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read rules.kdl: {}", e))?;

        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::with_capacity(lines.len() + 2);

        let match_pattern = format!("match namespace=\"{}\"", namespace);
        let value_str = if value { "true" } else { "false" };

        let mut i = 0;
        let mut patched = false;

        while i < lines.len() {
            // Look for layer-rule blocks
            if lines[i].trim().starts_with("layer-rule") && lines[i].trim().contains('{') {
                // Scan ahead to check if this block contains our namespace
                let block_start = i;
                let mut block_end = i;
                let mut depth = 0;
                let mut contains_namespace = false;

                for j in i..lines.len() {
                    let trimmed = lines[j].trim();
                    for ch in trimmed.chars() {
                        if ch == '{' { depth += 1; }
                        if ch == '}' { depth -= 1; }
                    }
                    if trimmed.contains(&match_pattern) {
                        contains_namespace = true;
                    }
                    if depth == 0 {
                        block_end = j;
                        break;
                    }
                }

                if contains_namespace {
                    // Found the target block — patch it
                    let block_lines: Vec<&str> = lines[block_start..=block_end].to_vec();
                    let patched_block = Self::patch_background_effect(&block_lines, property, value_str);
                    for line in patched_block {
                        result.push(line);
                    }
                    i = block_end + 1;
                    patched = true;
                    continue;
                }
            }

            result.push(lines[i].to_string());
            i += 1;
        }

        if !patched {
            return Err(format!("No layer-rule found for namespace '{}'", namespace));
        }

        let new_content = result.join("\n");
        // Preserve trailing newline if original had one
        let final_content = if content.ends_with('\n') {
            format!("{}\n", new_content)
        } else {
            new_content
        };

        fs::write(&path, &final_content)
            .map_err(|e| format!("Failed to write rules.kdl: {}", e))?;

        // Also keep repo rules.kdl in sync if it exists
        if let Some(home) = dirs::home_dir() {
            let repo_rules = home.join("COS_Niri/.config/niri/rules.kdl");
            if repo_rules.exists() && repo_rules != path {
                let _ = fs::write(repo_rules, &final_content);
            }
        }

        Ok(())
    }

    /// Patch or insert a property within the background-effect sub-block of a layer-rule
    fn patch_background_effect(block: &[&str], property: &str, value: &str) -> Vec<String> {
        let mut result = Vec::with_capacity(block.len() + 1);
        let mut in_bg_effect = false;
        let mut bg_depth = 0;
        let mut property_found = false;

        for &line in block {
            let trimmed = line.trim();

            if trimmed.starts_with("background-effect") && trimmed.contains('{') {
                in_bg_effect = true;
                bg_depth = 0;
                for ch in trimmed.chars() {
                    if ch == '{' { bg_depth += 1; }
                    if ch == '}' { bg_depth -= 1; }
                }
                result.push(line.to_string());
                continue;
            }

            if in_bg_effect {
                for ch in trimmed.chars() {
                    if ch == '{' { bg_depth += 1; }
                    if ch == '}' { bg_depth -= 1; }
                }

                // Check if this line has our property
                if trimmed.starts_with(property) {
                    // Replace the value
                    let indent = &line[..line.len() - trimmed.len()];
                    result.push(format!("{}{} {}", indent, property, value));
                    property_found = true;
                } else if bg_depth == 0 {
                    // Closing brace of background-effect
                    if !property_found {
                        // Insert property before closing brace
                        let indent = &line[..line.len() - trimmed.len()];
                        result.push(format!("{}    {} {}", indent, property, value));
                    }
                    in_bg_effect = false;
                    result.push(line.to_string());
                } else {
                    result.push(line.to_string());
                }
            } else {
                result.push(line.to_string());
            }
        }

        result
    }

    /// Call niri to reload its config
    fn reload_niri() {
        crate::services::worker::TaskWorker::dispatch(|| {
            let _ = Command::new("niri")
                .args(["msg", "action", "load-config-file"])
                .output();
            eprintln!("[niri_config] Niri config reloaded");
        });
    }
}
