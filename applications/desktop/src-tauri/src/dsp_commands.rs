//! DSP effect chain management commands
//!
//! Provides Tauri commands for configuring and managing the DSP effect chain
//! during playback. Effects are processed in series before upsampling.

use crate::playback::PlaybackManager;
use serde::{Deserialize, Serialize};
use soul_audio::effects::{CompressorSettings, EqBand, LimiterSettings};
use tauri::State;

/// Effect type identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EffectType {
    #[serde(rename = "eq")]
    Eq { bands: Vec<EqBandData> },
    #[serde(rename = "compressor")]
    Compressor { settings: CompressorData },
    #[serde(rename = "limiter")]
    Limiter { settings: LimiterData },
}

/// EQ band data for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EqBandData {
    pub frequency: f32,
    pub gain: f32,
    pub q: f32,
}

impl From<EqBand> for EqBandData {
    fn from(band: EqBand) -> Self {
        Self {
            frequency: band.frequency,
            gain: band.gain_db,
            q: band.q,
        }
    }
}

impl From<EqBandData> for EqBand {
    fn from(data: EqBandData) -> Self {
        EqBand::new(data.frequency, data.gain, data.q)
    }
}

/// Compressor settings for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompressorData {
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub knee_db: f32,
    pub makeup_gain_db: f32,
}

impl From<CompressorSettings> for CompressorData {
    fn from(settings: CompressorSettings) -> Self {
        Self {
            threshold_db: settings.threshold_db,
            ratio: settings.ratio,
            attack_ms: settings.attack_ms,
            release_ms: settings.release_ms,
            knee_db: settings.knee_db,
            makeup_gain_db: settings.makeup_gain_db,
        }
    }
}

impl From<CompressorData> for CompressorSettings {
    fn from(data: CompressorData) -> Self {
        CompressorSettings {
            threshold_db: data.threshold_db,
            ratio: data.ratio,
            attack_ms: data.attack_ms,
            release_ms: data.release_ms,
            knee_db: data.knee_db,
            makeup_gain_db: data.makeup_gain_db,
        }
    }
}

/// Limiter settings for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimiterData {
    pub threshold_db: f32,
    pub release_ms: f32,
}

impl From<LimiterSettings> for LimiterData {
    fn from(settings: LimiterSettings) -> Self {
        Self {
            threshold_db: settings.threshold_db,
            release_ms: settings.release_ms,
        }
    }
}

impl From<LimiterData> for LimiterSettings {
    fn from(data: LimiterData) -> Self {
        LimiterSettings {
            threshold_db: data.threshold_db,
            release_ms: data.release_ms,
        }
    }
}

/// Effect slot data for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectSlot {
    pub index: usize,
    pub effect: Option<EffectType>,
    pub enabled: bool,
}

/// Internal effect slot state (cloneable, doesn't include index)
#[derive(Debug, Clone)]
pub struct EffectSlotState {
    pub effect: EffectType,
    pub enabled: bool,
}

/// Get available effect types
#[tauri::command]
pub async fn get_available_effects() -> Result<Vec<String>, String> {
    Ok(vec![
        "eq".to_string(),
        "compressor".to_string(),
        "limiter".to_string(),
    ])
}

/// Get current DSP chain configuration
#[tauri::command]
pub async fn get_dsp_chain(
    #[allow(unused_variables)] playback: State<'_, PlaybackManager>,
) -> Result<Vec<EffectSlot>, String> {
    #[cfg(feature = "effects")]
    {
        let slots = playback.get_effect_slots()?;

        Ok((0..4)
            .map(|index| EffectSlot {
                index,
                effect: slots[index].as_ref().map(|s| s.effect.clone()),
                enabled: slots[index].as_ref().map(|s| s.enabled).unwrap_or(false),
            })
            .collect())
    }

    #[cfg(not(feature = "effects"))]
    {
        Ok(vec![
            EffectSlot {
                index: 0,
                effect: None,
                enabled: false,
            },
            EffectSlot {
                index: 1,
                effect: None,
                enabled: false,
            },
            EffectSlot {
                index: 2,
                effect: None,
                enabled: false,
            },
            EffectSlot {
                index: 3,
                effect: None,
                enabled: false,
            },
        ])
    }
}

/// Add effect to chain at specified slot
#[tauri::command]
pub async fn add_effect_to_chain(
    #[allow(unused_variables)] playback: State<'_, PlaybackManager>,
    slot_index: usize,
    #[allow(unused_variables)] effect: EffectType,
) -> Result<(), String> {
    // Validate slot index
    if slot_index >= 4 {
        return Err("Slot index must be 0-3".to_string());
    }

    #[cfg(feature = "effects")]
    {
        playback.set_effect_slot(
            slot_index,
            Some(EffectSlotState {
                effect,
                enabled: true,
            }),
        )?;
        eprintln!("[add_effect_to_chain] Slot {}: effect added", slot_index);
    }

    #[cfg(not(feature = "effects"))]
    {
        eprintln!("[add_effect_to_chain] Effects feature not enabled");
    }

    Ok(())
}

/// Remove effect from chain slot
#[tauri::command]
pub async fn remove_effect_from_chain(
    #[allow(unused_variables)] playback: State<'_, PlaybackManager>,
    slot_index: usize,
) -> Result<(), String> {
    // Validate slot index
    if slot_index >= 4 {
        return Err("Slot index must be 0-3".to_string());
    }

    #[cfg(feature = "effects")]
    {
        playback.set_effect_slot(slot_index, None)?;
        eprintln!("[remove_effect_from_chain] Slot {}: effect removed", slot_index);
    }

    #[cfg(not(feature = "effects"))]
    {
        eprintln!("[remove_effect_from_chain] Effects feature not enabled");
    }

    Ok(())
}

/// Enable/disable effect in chain slot
#[tauri::command]
pub async fn toggle_effect(
    #[allow(unused_variables)] playback: State<'_, PlaybackManager>,
    slot_index: usize,
    #[allow(unused_variables)] enabled: bool,
) -> Result<(), String> {
    // Validate slot index
    if slot_index >= 4 {
        return Err("Slot index must be 0-3".to_string());
    }

    #[cfg(feature = "effects")]
    {
        let slots = playback.get_effect_slots()?;
        if let Some(mut slot_state) = slots[slot_index].clone() {
            slot_state.enabled = enabled;
            playback.set_effect_slot(slot_index, Some(slot_state))?;
            eprintln!(
                "[toggle_effect] Slot {}: {}",
                slot_index,
                if enabled { "enabled" } else { "disabled" }
            );
        } else {
            return Err(format!("No effect at slot {}", slot_index));
        }
    }

    #[cfg(not(feature = "effects"))]
    {
        eprintln!("[toggle_effect] Effects feature not enabled");
    }

    Ok(())
}

/// Update effect parameters
#[tauri::command]
pub async fn update_effect_parameters(
    #[allow(unused_variables)] playback: State<'_, PlaybackManager>,
    slot_index: usize,
    #[allow(unused_variables)] effect: EffectType,
) -> Result<(), String> {
    // Validate slot index
    if slot_index >= 4 {
        return Err("Slot index must be 0-3".to_string());
    }

    #[cfg(feature = "effects")]
    {
        let slots = playback.get_effect_slots()?;
        if let Some(slot_state) = &slots[slot_index] {
            playback.set_effect_slot(
                slot_index,
                Some(EffectSlotState {
                    effect,
                    enabled: slot_state.enabled,
                }),
            )?;
            eprintln!("[update_effect_parameters] Slot {}: parameters updated", slot_index);
        } else {
            return Err(format!("No effect at slot {}", slot_index));
        }
    }

    #[cfg(not(feature = "effects"))]
    {
        eprintln!("[update_effect_parameters] Effects feature not enabled");
    }

    Ok(())
}

/// Clear entire DSP chain
#[tauri::command]
pub async fn clear_dsp_chain(
    #[allow(unused_variables)] playback: State<'_, PlaybackManager>,
) -> Result<(), String> {
    #[cfg(feature = "effects")]
    {
        for i in 0..4 {
            playback.set_effect_slot(i, None)?;
        }
        eprintln!("[clear_dsp_chain] All effects cleared");
    }

    #[cfg(not(feature = "effects"))]
    {
        eprintln!("[clear_dsp_chain] Effects feature not enabled");
    }

    Ok(())
}

/// Get EQ presets
#[tauri::command]
pub async fn get_eq_presets() -> Result<Vec<(String, Vec<EqBandData>)>, String> {
    Ok(vec![
        (
            "Flat".to_string(),
            vec![
                EqBandData {
                    frequency: 100.0,
                    gain: 0.0,
                    q: 1.0,
                },
                EqBandData {
                    frequency: 1000.0,
                    gain: 0.0,
                    q: 1.0,
                },
                EqBandData {
                    frequency: 10000.0,
                    gain: 0.0,
                    q: 1.0,
                },
            ],
        ),
        (
            "Bass Boost".to_string(),
            vec![
                EqBandData {
                    frequency: 60.0,
                    gain: 6.0,
                    q: 1.0,
                },
                EqBandData {
                    frequency: 200.0,
                    gain: 3.0,
                    q: 1.0,
                },
                EqBandData {
                    frequency: 1000.0,
                    gain: 0.0,
                    q: 1.0,
                },
            ],
        ),
        (
            "Treble Boost".to_string(),
            vec![
                EqBandData {
                    frequency: 1000.0,
                    gain: 0.0,
                    q: 1.0,
                },
                EqBandData {
                    frequency: 5000.0,
                    gain: 3.0,
                    q: 1.0,
                },
                EqBandData {
                    frequency: 12000.0,
                    gain: 6.0,
                    q: 1.0,
                },
            ],
        ),
    ])
}

/// Get compressor presets
#[tauri::command]
pub async fn get_compressor_presets() -> Result<Vec<(String, CompressorData)>, String> {
    Ok(vec![
        (
            "Gentle".to_string(),
            CompressorSettings::gentle().into(),
        ),
        (
            "Moderate".to_string(),
            CompressorSettings::moderate().into(),
        ),
        (
            "Aggressive".to_string(),
            CompressorSettings::aggressive().into(),
        ),
    ])
}

/// Get limiter presets
#[tauri::command]
pub async fn get_limiter_presets() -> Result<Vec<(String, LimiterData)>, String> {
    Ok(vec![
        ("Soft".to_string(), LimiterSettings::soft().into()),
        ("Default".to_string(), LimiterSettings::default().into()),
        (
            "Brickwall".to_string(),
            LimiterSettings::brickwall().into(),
        ),
    ])
}

// ===== DSP Chain Presets =====

/// DSP preset data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DspPreset {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_builtin: bool,
    pub effect_chain: Vec<EffectType>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Get all DSP chain presets for current user
#[tauri::command]
pub async fn get_dsp_chain_presets(
    app_state: State<'_, crate::app_state::AppState>,
) -> Result<Vec<DspPreset>, String> {
    let user_id: i64 = app_state
        .user_id
        .parse()
        .map_err(|e| format!("Invalid user ID: {}", e))?;

    let presets = sqlx::query!(
        r#"
        SELECT id, name, description, is_builtin, effect_chain, created_at, updated_at
        FROM dsp_presets
        WHERE user_id = ?
        ORDER BY is_builtin DESC, name ASC
        "#,
        user_id
    )
    .fetch_all(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to fetch presets: {}", e))?;

    let mut result = Vec::new();
    for preset in presets {
        let effect_chain: Vec<EffectType> = serde_json::from_str(&preset.effect_chain)
            .map_err(|e| format!("Failed to parse effect chain: {}", e))?;

        result.push(DspPreset {
            id: preset.id.unwrap_or(0), // Should always be present for rows from DB
            name: preset.name,
            description: preset.description,
            is_builtin: preset.is_builtin,
            effect_chain,
            created_at: preset.created_at,
            updated_at: preset.updated_at,
        });
    }

    Ok(result)
}

/// Save a DSP chain preset
#[tauri::command]
pub async fn save_dsp_chain_preset(
    name: String,
    description: Option<String>,
    effect_chain: Vec<EffectType>,
    app_state: State<'_, crate::app_state::AppState>,
) -> Result<i64, String> {
    let user_id: i64 = app_state
        .user_id
        .parse()
        .map_err(|e| format!("Invalid user ID: {}", e))?;

    let effect_chain_json = serde_json::to_string(&effect_chain)
        .map_err(|e| format!("Failed to serialize effect chain: {}", e))?;

    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query!(
        r#"
        INSERT INTO dsp_presets (user_id, name, description, is_builtin, effect_chain, created_at, updated_at)
        VALUES (?, ?, ?, 0, ?, ?, ?)
        ON CONFLICT(user_id, name) DO UPDATE SET
            description = excluded.description,
            effect_chain = excluded.effect_chain,
            updated_at = excluded.updated_at
        "#,
        user_id,
        name,
        description,
        effect_chain_json,
        now,
        now
    )
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to save preset: {}", e))?;

    Ok(result.last_insert_rowid())
}

/// Delete a DSP chain preset
#[tauri::command]
pub async fn delete_dsp_chain_preset(
    preset_id: i64,
    app_state: State<'_, crate::app_state::AppState>,
) -> Result<(), String> {
    let user_id: i64 = app_state
        .user_id
        .parse()
        .map_err(|e| format!("Invalid user ID: {}", e))?;

    // Prevent deletion of built-in presets
    let preset = sqlx::query!(
        r#"
        SELECT is_builtin FROM dsp_presets WHERE id = ? AND user_id = ?
        "#,
        preset_id,
        user_id
    )
    .fetch_optional(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to fetch preset: {}", e))?;

    match preset {
        None => return Err("Preset not found".to_string()),
        Some(p) if p.is_builtin => return Err("Cannot delete built-in preset".to_string()),
        _ => {}
    }

    sqlx::query!(
        r#"
        DELETE FROM dsp_presets WHERE id = ? AND user_id = ?
        "#,
        preset_id,
        user_id
    )
    .execute(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to delete preset: {}", e))?;

    Ok(())
}

/// Load a DSP chain preset (apply it to the effect chain)
#[tauri::command]
pub async fn load_dsp_chain_preset(
    preset_id: i64,
    playback: State<'_, PlaybackManager>,
    app_state: State<'_, crate::app_state::AppState>,
) -> Result<(), String> {
    let user_id: i64 = app_state
        .user_id
        .parse()
        .map_err(|e| format!("Invalid user ID: {}", e))?;

    let preset = sqlx::query!(
        r#"
        SELECT effect_chain FROM dsp_presets WHERE id = ? AND user_id = ?
        "#,
        preset_id,
        user_id
    )
    .fetch_optional(&*app_state.pool)
    .await
    .map_err(|e| format!("Failed to fetch preset: {}", e))?
    .ok_or("Preset not found")?;

    let effect_chain: Vec<EffectType> = serde_json::from_str(&preset.effect_chain)
        .map_err(|e| format!("Failed to parse effect chain: {}", e))?;

    // Clear existing chain
    clear_dsp_chain(playback.clone()).await?;

    // Add each effect to its slot
    for (slot_index, effect) in effect_chain.iter().enumerate() {
        add_effect_to_chain(playback.clone(), slot_index, effect.clone()).await?;
    }

    Ok(())
}
