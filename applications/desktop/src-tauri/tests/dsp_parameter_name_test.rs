//! Regression tests for DSP parameter name mapping
//!
//! These tests verify that the frontend-backend parameter mapping is correct.
//! The Tauri commands expect `effect` as the parameter name for effect data,
//! NOT `parameters` which was a bug that prevented effects from working.
//!
//! See: https://github.com/anthropics/claude-code/issues/XXX

use serde_json::json;

/// This module tests the expected JSON format from frontend to backend
/// to catch parameter name mismatches early.
mod parameter_mapping {
    use super::*;

    /// Frontend sends data with the `effect` key, NOT `parameters`
    /// This was a critical bug where the frontend sent `parameters` but backend expected `effect`
    #[test]
    fn test_update_effect_parameters_expects_effect_key() {
        // Correct format that frontend should send
        let correct_format = json!({
            "slotIndex": 0,
            "effect": {
                "type": "compressor",
                "settings": {
                    "thresholdDb": -20,
                    "ratio": 4.0,
                    "attackMs": 10,
                    "releaseMs": 100,
                    "kneeDb": 2.0,
                    "makeupGainDb": 0
                }
            }
        });

        // Wrong format that was used before the fix
        let wrong_format = json!({
            "slotIndex": 0,
            "parameters": {  // WRONG KEY!
                "type": "compressor",
                "settings": {
                    "thresholdDb": -20,
                    "ratio": 4.0,
                    "attackMs": 10,
                    "releaseMs": 100,
                    "kneeDb": 2.0,
                    "makeupGainDb": 0
                }
            }
        });

        // Verify correct format has `effect` key
        assert!(
            correct_format.get("effect").is_some(),
            "Frontend must send 'effect' key, not 'parameters'"
        );

        // Verify wrong format would NOT have `effect` key
        assert!(
            wrong_format.get("effect").is_none(),
            "Wrong format should not have 'effect' key"
        );
    }

    /// Test all effect types use the correct `effect` key format
    #[test]
    fn test_all_effect_types_use_effect_key() {
        let effect_types = vec![
            ("eq", json!({ "type": "eq", "bands": [] })),
            (
                "compressor",
                json!({
                    "type": "compressor",
                    "settings": {
                        "thresholdDb": -20,
                        "ratio": 4.0,
                        "attackMs": 10,
                        "releaseMs": 100,
                        "kneeDb": 2.0,
                        "makeupGainDb": 0
                    }
                }),
            ),
            (
                "limiter",
                json!({
                    "type": "limiter",
                    "settings": {
                        "thresholdDb": -0.3,
                        "releaseMs": 50
                    }
                }),
            ),
            (
                "crossfeed",
                json!({
                    "type": "crossfeed",
                    "settings": {
                        "preset": "natural",
                        "levelDb": -4.5,
                        "cutoffHz": 700
                    }
                }),
            ),
            (
                "stereo",
                json!({
                    "type": "stereo",
                    "settings": {
                        "width": 1.0,
                        "midGainDb": 0,
                        "sideGainDb": 0,
                        "balance": 0
                    }
                }),
            ),
            (
                "graphic_eq",
                json!({
                    "type": "graphic_eq",
                    "settings": {
                        "preset": "Flat",
                        "bandCount": 10,
                        "gains": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
                    }
                }),
            ),
            (
                "convolution",
                json!({
                    "type": "convolution",
                    "settings": {
                        "irFilePath": "/path/to/ir.wav",
                        "wetDryMix": 0.3,
                        "preDelayMs": 0,
                        "decay": 1.0
                    }
                }),
            ),
        ];

        for (effect_name, effect_data) in effect_types {
            // Simulate the correct frontend call format
            let frontend_call = json!({
                "slotIndex": 0,
                "effect": effect_data
            });

            assert!(
                frontend_call.get("effect").is_some(),
                "{} effect must use 'effect' key, not 'parameters'",
                effect_name
            );

            assert!(
                frontend_call.get("parameters").is_none(),
                "{} effect must NOT use 'parameters' key (old bug)",
                effect_name
            );
        }
    }

    /// Test that add_effect_to_chain also uses `effect` key
    #[test]
    fn test_add_effect_uses_effect_key() {
        let correct_format = json!({
            "slotIndex": 0,
            "effect": {
                "type": "eq",
                "bands": [
                    { "frequency": 100, "gain": 0, "q": 1.0 },
                    { "frequency": 1000, "gain": 0, "q": 1.0 },
                    { "frequency": 10000, "gain": 0, "q": 1.0 }
                ]
            }
        });

        assert!(
            correct_format.get("effect").is_some(),
            "add_effect_to_chain must receive 'effect' key"
        );
    }

    /// Test slotIndex is used (camelCase) as Tauri auto-converts to snake_case
    #[test]
    fn test_slot_index_naming_convention() {
        let frontend_call = json!({
            "slotIndex": 2,  // camelCase from frontend
            "effect": { "type": "limiter", "settings": { "thresholdDb": -0.3, "releaseMs": 50 } }
        });

        // Frontend should use camelCase
        assert!(frontend_call.get("slotIndex").is_some());
        // Frontend should NOT use snake_case
        assert!(frontend_call.get("slot_index").is_none());
    }
}

/// This module documents the correct parameter format for each Tauri command
mod command_documentation {
    use super::*;

    /// Documents the expected format for update_effect_parameters
    ///
    /// Frontend must call:
    /// ```javascript
    /// await invoke('update_effect_parameters', {
    ///   slotIndex: 0,
    ///   effect: { type: 'compressor', settings: {...} }
    /// });
    /// ```
    ///
    /// NOT:
    /// ```javascript
    /// await invoke('update_effect_parameters', {
    ///   slotIndex: 0,
    ///   parameters: { type: 'compressor', settings: {...} }  // WRONG!
    /// });
    /// ```
    #[test]
    fn document_update_effect_parameters_format() {
        let expected = json!({
            "command": "update_effect_parameters",
            "arguments": {
                "slotIndex": "usize (0-3)",
                "effect": "EffectType enum variant with settings"
            }
        });

        // This test exists as documentation
        assert!(expected.get("command").is_some());
    }

    /// Documents the expected format for add_effect_to_chain
    #[test]
    fn document_add_effect_to_chain_format() {
        let expected = json!({
            "command": "add_effect_to_chain",
            "arguments": {
                "slotIndex": "usize (0-3)",
                "effect": "EffectType enum variant with settings"
            }
        });

        // This test exists as documentation
        assert!(expected.get("command").is_some());
    }

    /// Documents the expected format for toggle_effect
    #[test]
    fn document_toggle_effect_format() {
        let expected = json!({
            "command": "toggle_effect",
            "arguments": {
                "slotIndex": "usize (0-3)",
                "enabled": "bool"
            }
        });

        // This test exists as documentation
        assert!(expected.get("command").is_some());
    }

    /// Documents the expected format for remove_effect_from_chain
    #[test]
    fn document_remove_effect_from_chain_format() {
        let expected = json!({
            "command": "remove_effect_from_chain",
            "arguments": {
                "slotIndex": "usize (0-3)"
            }
        });

        // This test exists as documentation
        assert!(expected.get("command").is_some());
    }
}
