//! DSP Effect Parameter Mapping Integration Tests
//!
//! Tests that effect parameters flow correctly between frontend JSON format,
//! backend Rust types, and audio engine format. Validates serialization,
//! deserialization, parameter clamping, and roundtrip conversions.

use serde_json::json;

// Import the dsp_commands module types for testing
// We need to reference the crate's internal types
mod dsp_types {
    use serde::{Deserialize, Serialize};
    use soul_audio::effects::{
        CompressorSettings, CrossfeedPreset, CrossfeedSettings, EqBand, GraphicEqPreset,
        LimiterSettings, StereoSettings,
    };

    /// Effect type identifier - mirrors dsp_commands.rs EffectType
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type", rename_all = "camelCase")]
    pub enum EffectType {
        #[serde(rename = "eq")]
        Eq { bands: Vec<EqBandData> },
        #[serde(rename = "compressor")]
        Compressor { settings: CompressorData },
        #[serde(rename = "limiter")]
        Limiter { settings: LimiterData },
        #[serde(rename = "crossfeed")]
        Crossfeed { settings: CrossfeedData },
        #[serde(rename = "stereo")]
        Stereo { settings: StereoData },
        #[serde(rename = "graphic_eq")]
        GraphicEq { settings: GraphicEqData },
        #[serde(rename = "convolution")]
        Convolution { settings: ConvolutionData },
    }

    /// Convolution reverb settings for frontend
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct ConvolutionData {
        pub ir_file_path: String,
        pub wet_dry_mix: f32,
        pub pre_delay_ms: f32,
        pub decay: f32,
    }

    impl Default for ConvolutionData {
        fn default() -> Self {
            Self {
                ir_file_path: String::new(),
                wet_dry_mix: 0.3,
                pre_delay_ms: 0.0,
                decay: 1.0,
            }
        }
    }

    /// EQ band data for frontend
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
                gain: band.gain_db(),
                q: band.q(),
            }
        }
    }

    impl From<EqBandData> for EqBand {
        fn from(data: EqBandData) -> Self {
            EqBand::new(data.frequency, data.gain, data.q)
        }
    }

    /// Compressor settings for frontend
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Crossfeed settings for frontend
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct CrossfeedData {
        pub preset: String,
        pub level_db: f32,
        pub cutoff_hz: f32,
    }

    impl From<CrossfeedSettings> for CrossfeedData {
        fn from(settings: CrossfeedSettings) -> Self {
            let preset = match settings.preset {
                CrossfeedPreset::Natural => "natural",
                CrossfeedPreset::Relaxed => "relaxed",
                CrossfeedPreset::Meier => "meier",
                CrossfeedPreset::Custom => "custom",
            };
            Self {
                preset: preset.to_string(),
                level_db: settings.level_db,
                cutoff_hz: settings.cutoff_hz,
            }
        }
    }

    impl From<CrossfeedData> for CrossfeedSettings {
        fn from(data: CrossfeedData) -> Self {
            let preset = match data.preset.as_str() {
                "natural" => CrossfeedPreset::Natural,
                "relaxed" => CrossfeedPreset::Relaxed,
                "meier" => CrossfeedPreset::Meier,
                _ => CrossfeedPreset::Custom,
            };
            if preset == CrossfeedPreset::Custom {
                CrossfeedSettings::custom(data.level_db, data.cutoff_hz)
            } else {
                CrossfeedSettings::from_preset(preset)
            }
        }
    }

    /// Stereo enhancer settings for frontend
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct StereoData {
        pub width: f32,
        pub mid_gain_db: f32,
        pub side_gain_db: f32,
        pub balance: f32,
    }

    impl From<StereoSettings> for StereoData {
        fn from(settings: StereoSettings) -> Self {
            Self {
                width: settings.width,
                mid_gain_db: settings.mid_gain_db,
                side_gain_db: settings.side_gain_db,
                balance: settings.balance,
            }
        }
    }

    impl From<StereoData> for StereoSettings {
        fn from(data: StereoData) -> Self {
            StereoSettings {
                width: data.width.clamp(0.0, 2.0),
                mid_gain_db: data.mid_gain_db.clamp(-12.0, 12.0),
                side_gain_db: data.side_gain_db.clamp(-12.0, 12.0),
                balance: data.balance.clamp(-1.0, 1.0),
            }
        }
    }

    /// Graphic EQ settings for frontend
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct GraphicEqData {
        pub preset: String,
        pub band_count: u8,
        pub gains: Vec<f32>,
    }

    impl GraphicEqData {
        pub fn from_preset(preset: GraphicEqPreset) -> Self {
            let gains = preset.gains_10().to_vec();
            Self {
                preset: preset.name().to_string(),
                band_count: 10,
                gains,
            }
        }

        pub fn flat_10() -> Self {
            Self {
                preset: "Flat".to_string(),
                band_count: 10,
                gains: vec![0.0; 10],
            }
        }
    }
}

use dsp_types::*;
use soul_audio::effects::{
    CompressorSettings, CrossfeedPreset, CrossfeedSettings, EqBand, GraphicEqPreset,
    LimiterSettings, StereoSettings,
};

// ============================================================================
// 1. SERIALIZATION/DESERIALIZATION TESTS
// ============================================================================

mod serialization {
    use super::*;

    #[test]
    fn test_compressor_json_deserialization() {
        // Frontend sends this JSON (camelCase)
        let json = json!({
            "type": "compressor",
            "settings": {
                "thresholdDb": -20.0,
                "ratio": 4.0,
                "attackMs": 10.0,
                "releaseMs": 100.0,
                "kneeDb": 2.0,
                "makeupGainDb": 0.0
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Compressor { settings } => {
                assert_eq!(settings.threshold_db, -20.0);
                assert_eq!(settings.ratio, 4.0);
                assert_eq!(settings.attack_ms, 10.0);
                assert_eq!(settings.release_ms, 100.0);
                assert_eq!(settings.knee_db, 2.0);
                assert_eq!(settings.makeup_gain_db, 0.0);
            }
            _ => panic!("Expected Compressor variant"),
        }
    }

    #[test]
    fn test_limiter_json_deserialization() {
        let json = json!({
            "type": "limiter",
            "settings": {
                "thresholdDb": -0.3,
                "releaseMs": 50.0
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Limiter { settings } => {
                assert_eq!(settings.threshold_db, -0.3);
                assert_eq!(settings.release_ms, 50.0);
            }
            _ => panic!("Expected Limiter variant"),
        }
    }

    #[test]
    fn test_eq_json_deserialization() {
        let json = json!({
            "type": "eq",
            "bands": [
                { "frequency": 100.0, "gain": 3.0, "q": 1.0 },
                { "frequency": 1000.0, "gain": 0.0, "q": 1.5 },
                { "frequency": 10000.0, "gain": -2.0, "q": 0.7 }
            ]
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Eq { bands } => {
                assert_eq!(bands.len(), 3);
                assert_eq!(bands[0].frequency, 100.0);
                assert_eq!(bands[0].gain, 3.0);
                assert_eq!(bands[0].q, 1.0);
                assert_eq!(bands[1].frequency, 1000.0);
                assert_eq!(bands[2].frequency, 10000.0);
                assert_eq!(bands[2].gain, -2.0);
            }
            _ => panic!("Expected Eq variant"),
        }
    }

    #[test]
    fn test_crossfeed_json_deserialization() {
        let json = json!({
            "type": "crossfeed",
            "settings": {
                "preset": "natural",
                "levelDb": -4.5,
                "cutoffHz": 700.0
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Crossfeed { settings } => {
                assert_eq!(settings.preset, "natural");
                assert_eq!(settings.level_db, -4.5);
                assert_eq!(settings.cutoff_hz, 700.0);
            }
            _ => panic!("Expected Crossfeed variant"),
        }
    }

    #[test]
    fn test_stereo_json_deserialization() {
        let json = json!({
            "type": "stereo",
            "settings": {
                "width": 1.5,
                "midGainDb": 2.0,
                "sideGainDb": -1.0,
                "balance": 0.2
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Stereo { settings } => {
                assert_eq!(settings.width, 1.5);
                assert_eq!(settings.mid_gain_db, 2.0);
                assert_eq!(settings.side_gain_db, -1.0);
                assert_eq!(settings.balance, 0.2);
            }
            _ => panic!("Expected Stereo variant"),
        }
    }

    #[test]
    fn test_graphic_eq_json_deserialization() {
        let json = json!({
            "type": "graphic_eq",
            "settings": {
                "preset": "Bass Boost",
                "bandCount": 10,
                "gains": [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::GraphicEq { settings } => {
                assert_eq!(settings.preset, "Bass Boost");
                assert_eq!(settings.band_count, 10);
                assert_eq!(settings.gains.len(), 10);
                assert_eq!(settings.gains[0], 6.0);
            }
            _ => panic!("Expected GraphicEq variant"),
        }
    }

    #[test]
    fn test_convolution_json_deserialization() {
        let json = json!({
            "type": "convolution",
            "settings": {
                "irFilePath": "/path/to/impulse_response.wav",
                "wetDryMix": 0.5,
                "preDelayMs": 10.0,
                "decay": 1.2
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Convolution { settings } => {
                assert_eq!(settings.ir_file_path, "/path/to/impulse_response.wav");
                assert_eq!(settings.wet_dry_mix, 0.5);
                assert_eq!(settings.pre_delay_ms, 10.0);
                assert_eq!(settings.decay, 1.2);
            }
            _ => panic!("Expected Convolution variant"),
        }
    }

    #[test]
    fn test_convolution_serialization_uses_camelcase() {
        let effect = EffectType::Convolution {
            settings: ConvolutionData {
                ir_file_path: "/path/to/ir.wav".to_string(),
                wet_dry_mix: 0.4,
                pre_delay_ms: 5.0,
                decay: 0.8,
            },
        };

        let json = serde_json::to_value(&effect).expect("Should serialize");

        // Verify camelCase keys in JSON
        let settings = json.get("settings").expect("Should have settings");
        assert!(settings.get("irFilePath").is_some());
        assert!(settings.get("wetDryMix").is_some());
        assert!(settings.get("preDelayMs").is_some());
        assert!(settings.get("decay").is_some());

        // Verify snake_case keys are NOT present
        assert!(settings.get("ir_file_path").is_none());
        assert!(settings.get("wet_dry_mix").is_none());
        assert!(settings.get("pre_delay_ms").is_none());
    }

    #[test]
    fn test_compressor_serialization_uses_camelcase() {
        let effect = EffectType::Compressor {
            settings: CompressorData {
                threshold_db: -15.0,
                ratio: 3.0,
                attack_ms: 5.0,
                release_ms: 75.0,
                knee_db: 4.0,
                makeup_gain_db: 2.0,
            },
        };

        let json = serde_json::to_value(&effect).expect("Should serialize");

        // Verify camelCase keys in JSON
        let settings = json.get("settings").expect("Should have settings");
        assert!(settings.get("thresholdDb").is_some());
        assert!(settings.get("ratio").is_some());
        assert!(settings.get("attackMs").is_some());
        assert!(settings.get("releaseMs").is_some());
        assert!(settings.get("kneeDb").is_some());
        assert!(settings.get("makeupGainDb").is_some());

        // Verify snake_case keys are NOT present
        assert!(settings.get("threshold_db").is_none());
        assert!(settings.get("attack_ms").is_none());
    }

    #[test]
    fn test_stereo_serialization_uses_camelcase() {
        let effect = EffectType::Stereo {
            settings: StereoData {
                width: 1.0,
                mid_gain_db: 0.0,
                side_gain_db: 0.0,
                balance: 0.0,
            },
        };

        let json = serde_json::to_value(&effect).expect("Should serialize");

        let settings = json.get("settings").expect("Should have settings");
        assert!(settings.get("width").is_some());
        assert!(settings.get("midGainDb").is_some());
        assert!(settings.get("sideGainDb").is_some());
        assert!(settings.get("balance").is_some());

        // Verify snake_case keys are NOT present
        assert!(settings.get("mid_gain_db").is_none());
        assert!(settings.get("side_gain_db").is_none());
    }

    #[test]
    fn test_all_effect_types_have_correct_type_tag() {
        // Test each effect type serializes with the correct "type" tag
        let test_cases = vec![
            (
                EffectType::Eq {
                    bands: vec![EqBandData {
                        frequency: 1000.0,
                        gain: 0.0,
                        q: 1.0,
                    }],
                },
                "eq",
            ),
            (
                EffectType::Compressor {
                    settings: CompressorData {
                        threshold_db: -20.0,
                        ratio: 4.0,
                        attack_ms: 5.0,
                        release_ms: 50.0,
                        knee_db: 6.0,
                        makeup_gain_db: 0.0,
                    },
                },
                "compressor",
            ),
            (
                EffectType::Limiter {
                    settings: LimiterData {
                        threshold_db: -0.3,
                        release_ms: 50.0,
                    },
                },
                "limiter",
            ),
            (
                EffectType::Crossfeed {
                    settings: CrossfeedData {
                        preset: "natural".to_string(),
                        level_db: -4.5,
                        cutoff_hz: 700.0,
                    },
                },
                "crossfeed",
            ),
            (
                EffectType::Stereo {
                    settings: StereoData {
                        width: 1.0,
                        mid_gain_db: 0.0,
                        side_gain_db: 0.0,
                        balance: 0.0,
                    },
                },
                "stereo",
            ),
            (
                EffectType::GraphicEq {
                    settings: GraphicEqData::flat_10(),
                },
                "graphic_eq",
            ),
            (
                EffectType::Convolution {
                    settings: ConvolutionData::default(),
                },
                "convolution",
            ),
        ];

        for (effect, expected_type) in test_cases {
            let json = serde_json::to_value(&effect).expect("Should serialize");
            let type_value = json.get("type").expect("Should have type");
            assert_eq!(
                type_value.as_str().unwrap(),
                expected_type,
                "Effect type tag mismatch"
            );
        }
    }
}

// ============================================================================
// 2. PARAMETER RANGE / CLAMPING TESTS
// ============================================================================

mod parameter_ranges {
    use super::*;

    #[test]
    fn test_eq_band_gain_clamping() {
        // EqBand clamps gain to -12 to +12 dB
        let band = EqBand::new(1000.0, 20.0, 1.0);
        assert_eq!(band.gain_db(), 12.0, "Gain should be clamped to 12.0");

        let band_low = EqBand::new(1000.0, -20.0, 1.0);
        assert_eq!(band_low.gain_db(), -12.0, "Gain should be clamped to -12.0");
    }

    #[test]
    fn test_eq_band_q_clamping() {
        // EqBand clamps Q to 0.1 to 10.0
        let band = EqBand::new(1000.0, 0.0, 0.01);
        assert_eq!(band.q(), 0.1, "Q should be clamped to 0.1");

        let band_high = EqBand::new(1000.0, 0.0, 100.0);
        assert_eq!(band_high.q(), 10.0, "Q should be clamped to 10.0");
    }

    #[test]
    fn test_compressor_settings_validation() {
        let mut settings = CompressorSettings {
            threshold_db: -100.0, // Out of range (min -60)
            ratio: 50.0,          // Out of range (max 20)
            attack_ms: 0.01,      // Out of range (min 0.1)
            release_ms: 5000.0,   // Out of range (max 1000)
            knee_db: 20.0,        // Out of range (max 10)
            makeup_gain_db: 50.0, // Out of range (max 24)
        };

        settings.validate();

        assert!(
            settings.threshold_db >= -60.0 && settings.threshold_db <= 0.0,
            "Threshold should be clamped"
        );
        assert!(
            settings.ratio >= 1.0 && settings.ratio <= 20.0,
            "Ratio should be clamped"
        );
        assert!(
            settings.attack_ms >= 0.1 && settings.attack_ms <= 100.0,
            "Attack should be clamped"
        );
        assert!(
            settings.release_ms >= 10.0 && settings.release_ms <= 1000.0,
            "Release should be clamped"
        );
        assert!(
            settings.knee_db >= 0.0 && settings.knee_db <= 10.0,
            "Knee should be clamped"
        );
        assert!(
            settings.makeup_gain_db >= 0.0 && settings.makeup_gain_db <= 24.0,
            "Makeup gain should be clamped"
        );
    }

    #[test]
    fn test_stereo_settings_clamping() {
        let data = StereoData {
            width: 5.0,        // Out of range (max 2.0)
            mid_gain_db: 20.0, // Out of range (max 12.0)
            side_gain_db: -20.0, // Out of range (min -12.0)
            balance: 2.0,      // Out of range (max 1.0)
        };

        let settings: StereoSettings = data.into();

        assert_eq!(settings.width, 2.0, "Width should be clamped to 2.0");
        assert_eq!(
            settings.mid_gain_db, 12.0,
            "Mid gain should be clamped to 12.0"
        );
        assert_eq!(
            settings.side_gain_db, -12.0,
            "Side gain should be clamped to -12.0"
        );
        assert_eq!(settings.balance, 1.0, "Balance should be clamped to 1.0");
    }

    #[test]
    fn test_stereo_settings_clamping_negative() {
        let data = StereoData {
            width: -1.0,
            mid_gain_db: -20.0,
            side_gain_db: 20.0,
            balance: -2.0,
        };

        let settings: StereoSettings = data.into();

        assert_eq!(settings.width, 0.0, "Width should be clamped to 0.0");
        assert_eq!(
            settings.mid_gain_db, -12.0,
            "Mid gain should be clamped to -12.0"
        );
        assert_eq!(
            settings.side_gain_db, 12.0,
            "Side gain should be clamped to 12.0"
        );
        assert_eq!(settings.balance, -1.0, "Balance should be clamped to -1.0");
    }

    #[test]
    fn test_crossfeed_custom_settings_clamping() {
        // CrossfeedSettings::custom clamps level_db to -12 to -3
        // and cutoff_hz to 300 to 1000
        let settings = CrossfeedSettings::custom(-20.0, 100.0);

        assert_eq!(settings.level_db, -12.0, "Level should be clamped to -12.0");
        assert_eq!(
            settings.cutoff_hz, 300.0,
            "Cutoff should be clamped to 300.0"
        );

        let settings_high = CrossfeedSettings::custom(-1.0, 2000.0);
        assert_eq!(settings_high.level_db, -3.0, "Level should be clamped to -3.0");
        assert_eq!(
            settings_high.cutoff_hz, 1000.0,
            "Cutoff should be clamped to 1000.0"
        );
    }

    #[test]
    fn test_compressor_boundary_values() {
        // Test exact boundary values
        let mut settings = CompressorSettings {
            threshold_db: -60.0, // Exact min
            ratio: 1.0,          // Exact min
            attack_ms: 0.1,      // Exact min
            release_ms: 10.0,    // Exact min
            knee_db: 0.0,        // Exact min
            makeup_gain_db: 0.0, // Exact min
        };
        settings.validate();
        assert_eq!(settings.threshold_db, -60.0);
        assert_eq!(settings.ratio, 1.0);
        assert_eq!(settings.attack_ms, 0.1);
        assert_eq!(settings.release_ms, 10.0);
        assert_eq!(settings.knee_db, 0.0);
        assert_eq!(settings.makeup_gain_db, 0.0);

        // Test exact max boundary
        let mut settings_max = CompressorSettings {
            threshold_db: 0.0,     // Exact max
            ratio: 20.0,           // Exact max
            attack_ms: 100.0,      // Exact max
            release_ms: 1000.0,    // Exact max
            knee_db: 10.0,         // Exact max
            makeup_gain_db: 24.0,  // Exact max
        };
        settings_max.validate();
        assert_eq!(settings_max.threshold_db, 0.0);
        assert_eq!(settings_max.ratio, 20.0);
        assert_eq!(settings_max.attack_ms, 100.0);
        assert_eq!(settings_max.release_ms, 1000.0);
        assert_eq!(settings_max.knee_db, 10.0);
        assert_eq!(settings_max.makeup_gain_db, 24.0);
    }
}

// ============================================================================
// 3. EFFECT TYPE CONVERSION TESTS
// ============================================================================

mod type_conversions {
    use super::*;

    #[test]
    fn test_eq_band_data_to_eq_band() {
        let data = EqBandData {
            frequency: 1000.0,
            gain: 3.5,
            q: 1.4,
        };

        let band: EqBand = data.into();

        assert_eq!(band.frequency, 1000.0);
        assert_eq!(band.gain_db(), 3.5);
        assert_eq!(band.q(), 1.4);
    }

    #[test]
    fn test_eq_band_to_eq_band_data() {
        let band = EqBand::new(2000.0, -2.5, 2.0);

        let data: EqBandData = band.into();

        assert_eq!(data.frequency, 2000.0);
        assert_eq!(data.gain, -2.5);
        assert_eq!(data.q, 2.0);
    }

    #[test]
    fn test_compressor_data_to_compressor_settings() {
        let data = CompressorData {
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 4.0,
        };

        let settings: CompressorSettings = data.into();

        assert_eq!(settings.threshold_db, -18.0);
        assert_eq!(settings.ratio, 4.0);
        assert_eq!(settings.attack_ms, 5.0);
        assert_eq!(settings.release_ms, 50.0);
        assert_eq!(settings.knee_db, 6.0);
        assert_eq!(settings.makeup_gain_db, 4.0);
    }

    #[test]
    fn test_compressor_settings_to_compressor_data() {
        let settings = CompressorSettings::moderate();

        let data: CompressorData = settings.into();

        assert_eq!(data.threshold_db, settings.threshold_db);
        assert_eq!(data.ratio, settings.ratio);
        assert_eq!(data.attack_ms, settings.attack_ms);
        assert_eq!(data.release_ms, settings.release_ms);
        assert_eq!(data.knee_db, settings.knee_db);
        assert_eq!(data.makeup_gain_db, settings.makeup_gain_db);
    }

    #[test]
    fn test_limiter_data_to_limiter_settings() {
        let data = LimiterData {
            threshold_db: -0.5,
            release_ms: 75.0,
        };

        let settings: LimiterSettings = data.into();

        assert_eq!(settings.threshold_db, -0.5);
        assert_eq!(settings.release_ms, 75.0);
    }

    #[test]
    fn test_limiter_settings_to_limiter_data() {
        let settings = LimiterSettings::brickwall();

        let data: LimiterData = settings.into();

        assert_eq!(data.threshold_db, settings.threshold_db);
        assert_eq!(data.release_ms, settings.release_ms);
    }

    #[test]
    fn test_crossfeed_data_to_crossfeed_settings_preset() {
        // Test each preset
        let natural_data = CrossfeedData {
            preset: "natural".to_string(),
            level_db: -4.5,
            cutoff_hz: 700.0,
        };
        let natural_settings: CrossfeedSettings = natural_data.into();
        assert_eq!(natural_settings.preset, CrossfeedPreset::Natural);

        let relaxed_data = CrossfeedData {
            preset: "relaxed".to_string(),
            level_db: -6.0,
            cutoff_hz: 650.0,
        };
        let relaxed_settings: CrossfeedSettings = relaxed_data.into();
        assert_eq!(relaxed_settings.preset, CrossfeedPreset::Relaxed);

        let meier_data = CrossfeedData {
            preset: "meier".to_string(),
            level_db: -9.0,
            cutoff_hz: 550.0,
        };
        let meier_settings: CrossfeedSettings = meier_data.into();
        assert_eq!(meier_settings.preset, CrossfeedPreset::Meier);
    }

    #[test]
    fn test_crossfeed_data_to_crossfeed_settings_custom() {
        let data = CrossfeedData {
            preset: "custom".to_string(),
            level_db: -8.0,
            cutoff_hz: 600.0,
        };

        let settings: CrossfeedSettings = data.into();

        assert_eq!(settings.preset, CrossfeedPreset::Custom);
        assert_eq!(settings.level_db, -8.0);
        assert_eq!(settings.cutoff_hz, 600.0);
    }

    #[test]
    fn test_crossfeed_settings_to_crossfeed_data() {
        let settings = CrossfeedSettings::from_preset(CrossfeedPreset::Natural);

        let data: CrossfeedData = settings.into();

        assert_eq!(data.preset, "natural");
        assert_eq!(data.level_db, -4.5);
        assert_eq!(data.cutoff_hz, 700.0);
    }

    #[test]
    fn test_stereo_data_to_stereo_settings() {
        let data = StereoData {
            width: 1.5,
            mid_gain_db: 2.0,
            side_gain_db: -1.0,
            balance: 0.3,
        };

        let settings: StereoSettings = data.into();

        assert_eq!(settings.width, 1.5);
        assert_eq!(settings.mid_gain_db, 2.0);
        assert_eq!(settings.side_gain_db, -1.0);
        assert_eq!(settings.balance, 0.3);
    }

    #[test]
    fn test_stereo_settings_to_stereo_data() {
        let settings = StereoSettings::wide();

        // Get values before conversion since StereoSettings doesn't implement Copy
        let width = settings.width;
        let mid_gain_db = settings.mid_gain_db;
        let side_gain_db = settings.side_gain_db;
        let balance = settings.balance;

        let data: StereoData = settings.into();

        assert_eq!(data.width, width);
        assert_eq!(data.mid_gain_db, mid_gain_db);
        assert_eq!(data.side_gain_db, side_gain_db);
        assert_eq!(data.balance, balance);
    }

    #[test]
    fn test_graphic_eq_data_from_preset() {
        let data = GraphicEqData::from_preset(GraphicEqPreset::BassBoost);

        assert_eq!(data.preset, "Bass Boost");
        assert_eq!(data.band_count, 10);
        assert_eq!(data.gains.len(), 10);
        // Verify BassBoost gains pattern
        assert_eq!(data.gains[0], 6.0); // Low boost
        assert_eq!(data.gains[5], 0.0); // No boost at higher frequencies
    }

    #[test]
    fn test_graphic_eq_data_flat_10() {
        let data = GraphicEqData::flat_10();

        assert_eq!(data.preset, "Flat");
        assert_eq!(data.band_count, 10);
        assert_eq!(data.gains.len(), 10);
        for gain in &data.gains {
            assert_eq!(*gain, 0.0);
        }
    }
}

// ============================================================================
// 4. FULL ROUNDTRIP TESTS
// ============================================================================

mod roundtrip {
    use super::*;

    #[test]
    fn test_eq_band_roundtrip() {
        let original = EqBandData {
            frequency: 1000.0,
            gain: 3.5,
            q: 1.4,
        };

        // Serialize to JSON (what frontend receives)
        let json = serde_json::to_value(&original).expect("Should serialize");

        // Verify JSON structure
        assert!(json.get("frequency").is_some());
        assert!(json.get("gain").is_some());
        assert!(json.get("q").is_some());

        // Deserialize back
        let restored: EqBandData = serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(original.frequency, restored.frequency);
        assert_eq!(original.gain, restored.gain);
        assert_eq!(original.q, restored.q);
    }

    #[test]
    fn test_compressor_full_roundtrip() {
        // 1. Start with frontend JSON
        let frontend_json = json!({
            "type": "compressor",
            "settings": {
                "thresholdDb": -25.0,
                "ratio": 6.0,
                "attackMs": 8.0,
                "releaseMs": 80.0,
                "kneeDb": 3.0,
                "makeupGainDb": 5.0
            }
        });

        // 2. Deserialize to EffectType
        let effect: EffectType =
            serde_json::from_value(frontend_json.clone()).expect("Should deserialize");

        // 3. Extract CompressorData
        let original_data = match &effect {
            EffectType::Compressor { settings } => settings.clone(),
            _ => panic!("Expected Compressor"),
        };

        // 4. Convert to audio engine settings
        let audio_settings: CompressorSettings = original_data.clone().into();

        // 5. Convert back to CompressorData
        let roundtrip_data: CompressorData = audio_settings.into();

        // 6. Serialize back to JSON
        let roundtrip_effect = EffectType::Compressor {
            settings: roundtrip_data.clone(),
        };
        let roundtrip_json = serde_json::to_value(&roundtrip_effect).expect("Should serialize");

        // 7. Verify values match
        assert_eq!(roundtrip_data.threshold_db, original_data.threshold_db);
        assert_eq!(roundtrip_data.ratio, original_data.ratio);
        assert_eq!(roundtrip_data.attack_ms, original_data.attack_ms);
        assert_eq!(roundtrip_data.release_ms, original_data.release_ms);
        assert_eq!(roundtrip_data.knee_db, original_data.knee_db);
        assert_eq!(roundtrip_data.makeup_gain_db, original_data.makeup_gain_db);

        // 8. Verify JSON structure is identical
        assert_eq!(
            frontend_json.get("type"),
            roundtrip_json.get("type"),
            "Type tag should match"
        );
    }

    #[test]
    fn test_limiter_full_roundtrip() {
        let original = LimiterData {
            threshold_db: -0.5,
            release_ms: 75.0,
        };

        // Convert to audio settings
        let audio_settings: LimiterSettings = original.clone().into();

        // Convert back
        let roundtrip: LimiterData = audio_settings.into();

        assert_eq!(original.threshold_db, roundtrip.threshold_db);
        assert_eq!(original.release_ms, roundtrip.release_ms);
    }

    #[test]
    fn test_crossfeed_preset_roundtrip() {
        // Test Natural preset
        let original_settings = CrossfeedSettings::from_preset(CrossfeedPreset::Natural);

        // Convert to frontend data
        let data: CrossfeedData = original_settings.clone().into();

        // Convert back to settings
        let roundtrip_settings: CrossfeedSettings = data.into();

        assert_eq!(original_settings.preset, roundtrip_settings.preset);
        // Note: When using presets, level_db and cutoff_hz come from the preset
        assert_eq!(original_settings.level_db, roundtrip_settings.level_db);
        assert_eq!(original_settings.cutoff_hz, roundtrip_settings.cutoff_hz);
    }

    #[test]
    fn test_crossfeed_custom_roundtrip() {
        let original_settings = CrossfeedSettings::custom(-7.5, 625.0);

        let data: CrossfeedData = original_settings.clone().into();
        let roundtrip_settings: CrossfeedSettings = data.into();

        assert_eq!(roundtrip_settings.preset, CrossfeedPreset::Custom);
        assert_eq!(original_settings.level_db, roundtrip_settings.level_db);
        assert_eq!(original_settings.cutoff_hz, roundtrip_settings.cutoff_hz);
    }

    #[test]
    fn test_stereo_full_roundtrip() {
        let frontend_json = json!({
            "type": "stereo",
            "settings": {
                "width": 1.3,
                "midGainDb": 1.5,
                "sideGainDb": -0.5,
                "balance": -0.2
            }
        });

        let effect: EffectType =
            serde_json::from_value(frontend_json).expect("Should deserialize");

        let original_data = match &effect {
            EffectType::Stereo { settings } => settings.clone(),
            _ => panic!("Expected Stereo"),
        };

        // Convert to audio engine settings
        let audio_settings: StereoSettings = original_data.clone().into();

        // Convert back
        let roundtrip_data: StereoData = audio_settings.into();

        assert_eq!(original_data.width, roundtrip_data.width);
        assert_eq!(original_data.mid_gain_db, roundtrip_data.mid_gain_db);
        assert_eq!(original_data.side_gain_db, roundtrip_data.side_gain_db);
        assert_eq!(original_data.balance, roundtrip_data.balance);
    }

    #[test]
    fn test_effect_chain_json_roundtrip() {
        // Create a complete effect chain as JSON (like the frontend would send)
        let chain_json = json!([
            {
                "type": "eq",
                "bands": [
                    { "frequency": 80.0, "gain": 3.0, "q": 0.707 },
                    { "frequency": 1000.0, "gain": -1.0, "q": 1.0 },
                    { "frequency": 8000.0, "gain": 2.0, "q": 0.707 }
                ]
            },
            {
                "type": "compressor",
                "settings": {
                    "thresholdDb": -18.0,
                    "ratio": 4.0,
                    "attackMs": 5.0,
                    "releaseMs": 50.0,
                    "kneeDb": 6.0,
                    "makeupGainDb": 4.0
                }
            },
            {
                "type": "limiter",
                "settings": {
                    "thresholdDb": -0.3,
                    "releaseMs": 50.0
                }
            }
        ]);

        // Deserialize the chain
        let effects: Vec<EffectType> =
            serde_json::from_value(chain_json.clone()).expect("Should deserialize chain");

        assert_eq!(effects.len(), 3);

        // Verify each effect type
        assert!(matches!(effects[0], EffectType::Eq { .. }));
        assert!(matches!(effects[1], EffectType::Compressor { .. }));
        assert!(matches!(effects[2], EffectType::Limiter { .. }));

        // Serialize back
        let roundtrip_json = serde_json::to_value(&effects).expect("Should serialize");

        // Verify we can deserialize again
        let _effects2: Vec<EffectType> =
            serde_json::from_value(roundtrip_json).expect("Should deserialize again");
    }

    #[test]
    fn test_persisted_effect_slot_structure() {
        // Test the structure used for database persistence
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PersistedEffectSlot {
            index: usize,
            effect: Option<EffectType>,
            enabled: bool,
        }

        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PersistedDspChain {
            slots: Vec<PersistedEffectSlot>,
        }

        let chain = PersistedDspChain {
            slots: vec![
                PersistedEffectSlot {
                    index: 0,
                    effect: Some(EffectType::Compressor {
                        settings: CompressorData {
                            threshold_db: -20.0,
                            ratio: 4.0,
                            attack_ms: 5.0,
                            release_ms: 50.0,
                            knee_db: 6.0,
                            makeup_gain_db: 0.0,
                        },
                    }),
                    enabled: true,
                },
                PersistedEffectSlot {
                    index: 1,
                    effect: Some(EffectType::Limiter {
                        settings: LimiterData {
                            threshold_db: -0.3,
                            release_ms: 50.0,
                        },
                    }),
                    enabled: true,
                },
                PersistedEffectSlot {
                    index: 2,
                    effect: None,
                    enabled: false,
                },
                PersistedEffectSlot {
                    index: 3,
                    effect: None,
                    enabled: false,
                },
            ],
        };

        // Serialize to JSON (as stored in database)
        let json = serde_json::to_value(&chain).expect("Should serialize");

        // Verify structure
        let slots = json.get("slots").expect("Should have slots array");
        assert!(slots.is_array());
        assert_eq!(slots.as_array().unwrap().len(), 4);

        // Deserialize back
        let restored: PersistedDspChain =
            serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(restored.slots.len(), 4);
        assert!(restored.slots[0].effect.is_some());
        assert!(restored.slots[1].effect.is_some());
        assert!(restored.slots[2].effect.is_none());
        assert!(restored.slots[3].effect.is_none());
    }
}

// ============================================================================
// 5. PRESET TESTS
// ============================================================================

mod presets {
    use super::*;

    #[test]
    fn test_compressor_presets_have_increasing_ratio() {
        let gentle = CompressorSettings::gentle();
        let moderate = CompressorSettings::moderate();
        let aggressive = CompressorSettings::aggressive();

        assert!(
            gentle.ratio < moderate.ratio,
            "Gentle ratio should be less than moderate"
        );
        assert!(
            moderate.ratio < aggressive.ratio,
            "Moderate ratio should be less than aggressive"
        );
    }

    #[test]
    fn test_limiter_presets_threshold_ordering() {
        // Limiter thresholds are:
        // - soft: -1.0 dB (more headroom before limiting)
        // - default: -0.3 dB
        // - brickwall: -0.1 dB (closest to 0 dB, most aggressive)
        // Lower threshold = more headroom, less aggressive
        // Higher threshold (closer to 0) = less headroom, more aggressive
        let soft = LimiterSettings::soft();
        let default = LimiterSettings::default();
        let brickwall = LimiterSettings::brickwall();

        assert!(
            soft.threshold_db < default.threshold_db,
            "Soft threshold (-1.0) should be lower than default (-0.3)"
        );
        assert!(
            default.threshold_db < brickwall.threshold_db,
            "Default threshold (-0.3) should be lower than brickwall (-0.1)"
        );

        // Verify actual values
        assert_eq!(soft.threshold_db, -1.0);
        assert_eq!(default.threshold_db, -0.3);
        assert_eq!(brickwall.threshold_db, -0.1);
    }

    #[test]
    fn test_crossfeed_preset_values() {
        assert_eq!(CrossfeedPreset::Natural.level_db(), -4.5);
        assert_eq!(CrossfeedPreset::Natural.cutoff_hz(), 700.0);

        assert_eq!(CrossfeedPreset::Relaxed.level_db(), -6.0);
        assert_eq!(CrossfeedPreset::Relaxed.cutoff_hz(), 650.0);

        assert_eq!(CrossfeedPreset::Meier.level_db(), -9.0);
        assert_eq!(CrossfeedPreset::Meier.cutoff_hz(), 550.0);
    }

    #[test]
    fn test_stereo_presets() {
        let mono = StereoSettings::mono();
        assert_eq!(mono.width, 0.0, "Mono should have width 0");

        let wide = StereoSettings::wide();
        assert_eq!(wide.width, 1.5, "Wide should have width 1.5");

        let extra_wide = StereoSettings::extra_wide();
        assert_eq!(extra_wide.width, 2.0, "Extra wide should have width 2.0");
    }

    #[test]
    fn test_graphic_eq_preset_gains() {
        let flat = GraphicEqPreset::Flat.gains_10();
        for gain in flat {
            assert_eq!(gain, 0.0, "Flat preset should have all zero gains");
        }

        let bass_boost = GraphicEqPreset::BassBoost.gains_10();
        assert!(
            bass_boost[0] > 0.0,
            "Bass boost should have positive first band"
        );
        assert!(
            bass_boost[0] > bass_boost[5],
            "Bass boost should have higher low freq gains"
        );

        let treble_boost = GraphicEqPreset::TrebleBoost.gains_10();
        assert!(
            treble_boost[9] > 0.0,
            "Treble boost should have positive last band"
        );
        assert!(
            treble_boost[9] > treble_boost[0],
            "Treble boost should have higher high freq gains"
        );
    }

    #[test]
    fn test_graphic_eq_preset_names() {
        assert_eq!(GraphicEqPreset::Flat.name(), "Flat");
        assert_eq!(GraphicEqPreset::BassBoost.name(), "Bass Boost");
        assert_eq!(GraphicEqPreset::TrebleBoost.name(), "Treble Boost");
        assert_eq!(GraphicEqPreset::VShape.name(), "V-Shape");
        assert_eq!(GraphicEqPreset::Vocal.name(), "Vocal");
        assert_eq!(GraphicEqPreset::Rock.name(), "Rock");
        assert_eq!(GraphicEqPreset::Electronic.name(), "Electronic");
        assert_eq!(GraphicEqPreset::Acoustic.name(), "Acoustic");
        assert_eq!(GraphicEqPreset::Custom.name(), "Custom");
    }
}

// ============================================================================
// 6. EDGE CASE TESTS
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_eq_bands() {
        let json = json!({
            "type": "eq",
            "bands": []
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::Eq { bands } => {
                assert_eq!(bands.len(), 0, "Should allow empty bands");
            }
            _ => panic!("Expected Eq variant"),
        }
    }

    #[test]
    fn test_graphic_eq_empty_gains() {
        let json = json!({
            "type": "graphic_eq",
            "settings": {
                "preset": "Custom",
                "bandCount": 0,
                "gains": []
            }
        });

        let effect: EffectType = serde_json::from_value(json).expect("Should deserialize");

        match effect {
            EffectType::GraphicEq { settings } => {
                assert_eq!(settings.gains.len(), 0);
                assert_eq!(settings.band_count, 0);
            }
            _ => panic!("Expected GraphicEq variant"),
        }
    }

    #[test]
    fn test_unknown_crossfeed_preset_becomes_custom() {
        let data = CrossfeedData {
            preset: "unknown_preset".to_string(),
            level_db: -6.0,
            cutoff_hz: 600.0,
        };

        let settings: CrossfeedSettings = data.into();

        assert_eq!(
            settings.preset,
            CrossfeedPreset::Custom,
            "Unknown preset should become Custom"
        );
    }

    #[test]
    fn test_float_precision_in_roundtrip() {
        // Test that we don't lose precision with typical float values
        let original = CompressorData {
            threshold_db: -18.5,
            ratio: 3.333,
            attack_ms: 2.5,
            release_ms: 75.0,
            knee_db: 4.0,
            makeup_gain_db: 1.5,
        };

        let json = serde_json::to_value(&original).expect("Should serialize");
        let restored: CompressorData = serde_json::from_value(json).expect("Should deserialize");

        // Allow small floating point differences
        assert!((original.threshold_db - restored.threshold_db).abs() < 0.001);
        assert!((original.ratio - restored.ratio).abs() < 0.001);
        assert!((original.attack_ms - restored.attack_ms).abs() < 0.001);
        assert!((original.release_ms - restored.release_ms).abs() < 0.001);
        assert!((original.knee_db - restored.knee_db).abs() < 0.001);
        assert!((original.makeup_gain_db - restored.makeup_gain_db).abs() < 0.001);
    }

    #[test]
    fn test_negative_and_zero_values() {
        let stereo_data = StereoData {
            width: 0.0,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 0.0,
        };

        let json = serde_json::to_value(&stereo_data).expect("Should serialize");
        let restored: StereoData = serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(restored.width, 0.0);
        assert_eq!(restored.mid_gain_db, 0.0);
        assert_eq!(restored.side_gain_db, 0.0);
        assert_eq!(restored.balance, 0.0);
    }

    #[test]
    fn test_eq_band_extreme_frequencies() {
        // Test very low frequency
        let low_band = EqBand::new(20.0, 0.0, 1.0);
        assert_eq!(low_band.frequency, 20.0);

        // Test very high frequency
        let high_band = EqBand::new(20000.0, 0.0, 1.0);
        assert_eq!(high_band.frequency, 20000.0);

        // Test DC (0 Hz) - should work, though not useful
        let dc_band = EqBand::new(0.0, 0.0, 1.0);
        assert_eq!(dc_band.frequency, 0.0);
    }

    #[test]
    fn test_compressor_unity_ratio() {
        // Ratio of 1.0 means no compression
        let data = CompressorData {
            threshold_db: -20.0,
            ratio: 1.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 0.0,
            makeup_gain_db: 0.0,
        };

        let settings: CompressorSettings = data.into();
        assert_eq!(settings.ratio, 1.0);
    }
}

// ============================================================================
// 7. INTEGRATION WITH AUDIO ENGINE
// ============================================================================

mod audio_engine_integration {
    use super::*;
    use soul_audio::effects::{Compressor, Limiter, ParametricEq, StereoEnhancer};

    #[test]
    fn test_compressor_data_creates_valid_effect() {
        let data = CompressorData {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            knee_db: 6.0,
            makeup_gain_db: 0.0,
        };

        let settings: CompressorSettings = data.into();
        let compressor = Compressor::with_settings(settings);

        // Verify the effect was created with correct settings
        let retrieved = compressor.settings();
        assert_eq!(retrieved.threshold_db, -20.0);
        assert_eq!(retrieved.ratio, 4.0);
    }

    #[test]
    fn test_limiter_data_creates_valid_effect() {
        let data = LimiterData {
            threshold_db: -0.3,
            release_ms: 50.0,
        };

        let settings: LimiterSettings = data.into();
        let limiter = Limiter::with_settings(settings);

        let retrieved = limiter.settings();
        assert_eq!(retrieved.threshold_db, -0.3);
        assert_eq!(retrieved.release_ms, 50.0);
    }

    #[test]
    fn test_eq_band_data_creates_valid_band() {
        let data = EqBandData {
            frequency: 1000.0,
            gain: 3.0,
            q: 1.4,
        };

        let band: EqBand = data.into();

        // Use the band to create an EQ
        let mut eq = ParametricEq::new();
        eq.set_mid_band(band);

        let retrieved = eq.mid_band();
        assert_eq!(retrieved.frequency, 1000.0);
        assert_eq!(retrieved.gain_db(), 3.0);
        assert_eq!(retrieved.q(), 1.4);
    }

    #[test]
    fn test_stereo_data_creates_valid_effect() {
        let data = StereoData {
            width: 1.5,
            mid_gain_db: 0.0,
            side_gain_db: 0.0,
            balance: 0.0,
        };

        let settings: StereoSettings = data.into();
        let enhancer = StereoEnhancer::with_settings(settings);

        assert_eq!(enhancer.width(), 1.5);
    }

    #[test]
    fn test_effect_type_to_audio_effect_conversion() {
        // Simulate the conversion that happens in the playback manager
        let effect_json = json!({
            "type": "compressor",
            "settings": {
                "thresholdDb": -18.0,
                "ratio": 4.0,
                "attackMs": 5.0,
                "releaseMs": 50.0,
                "kneeDb": 6.0,
                "makeupGainDb": 4.0
            }
        });

        let effect_type: EffectType =
            serde_json::from_value(effect_json).expect("Should deserialize");

        match effect_type {
            EffectType::Compressor { settings } => {
                let audio_settings: CompressorSettings = settings.into();
                let compressor = Compressor::with_settings(audio_settings);

                // Verify effect was created successfully with correct settings
                assert_eq!(compressor.settings().threshold_db, -18.0);
                assert_eq!(compressor.settings().ratio, 4.0);
                assert_eq!(compressor.settings().attack_ms, 5.0);
                assert_eq!(compressor.settings().release_ms, 50.0);
                assert_eq!(compressor.settings().knee_db, 6.0);
                assert_eq!(compressor.settings().makeup_gain_db, 4.0);
            }
            _ => panic!("Expected Compressor"),
        }
    }
}

// ============================================================================
// 8. CONVOLUTION EFFECT TESTS
// ============================================================================

mod convolution_tests {
    use super::*;

    #[test]
    fn test_convolution_data_default_values() {
        let data = ConvolutionData::default();

        assert_eq!(data.ir_file_path, "");
        assert_eq!(data.wet_dry_mix, 0.3);
        assert_eq!(data.pre_delay_ms, 0.0);
        assert_eq!(data.decay, 1.0);
    }

    #[test]
    fn test_convolution_roundtrip() {
        let original = ConvolutionData {
            ir_file_path: "/audio/impulse_responses/hall.wav".to_string(),
            wet_dry_mix: 0.45,
            pre_delay_ms: 25.0,
            decay: 1.5,
        };

        // Serialize to JSON
        let json = serde_json::to_value(&original).expect("Should serialize");

        // Verify JSON structure
        assert!(json.get("irFilePath").is_some());
        assert!(json.get("wetDryMix").is_some());
        assert!(json.get("preDelayMs").is_some());
        assert!(json.get("decay").is_some());

        // Deserialize back
        let restored: ConvolutionData = serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(original.ir_file_path, restored.ir_file_path);
        assert!((original.wet_dry_mix - restored.wet_dry_mix).abs() < 0.001);
        assert!((original.pre_delay_ms - restored.pre_delay_ms).abs() < 0.001);
        assert!((original.decay - restored.decay).abs() < 0.001);
    }

    #[test]
    fn test_convolution_full_effect_roundtrip() {
        let frontend_json = json!({
            "type": "convolution",
            "settings": {
                "irFilePath": "/path/to/reverb.wav",
                "wetDryMix": 0.35,
                "preDelayMs": 15.0,
                "decay": 0.9
            }
        });

        // Deserialize to EffectType
        let effect: EffectType =
            serde_json::from_value(frontend_json.clone()).expect("Should deserialize");

        // Extract ConvolutionData
        let original_data = match &effect {
            EffectType::Convolution { settings } => settings.clone(),
            _ => panic!("Expected Convolution"),
        };

        // Serialize back to JSON
        let roundtrip_effect = EffectType::Convolution {
            settings: original_data.clone(),
        };
        let roundtrip_json = serde_json::to_value(&roundtrip_effect).expect("Should serialize");

        // Verify type tag matches
        assert_eq!(
            frontend_json.get("type"),
            roundtrip_json.get("type"),
            "Type tag should match"
        );

        // Verify values preserved
        assert_eq!(original_data.ir_file_path, "/path/to/reverb.wav");
        assert_eq!(original_data.wet_dry_mix, 0.35);
        assert_eq!(original_data.pre_delay_ms, 15.0);
        assert_eq!(original_data.decay, 0.9);
    }

    #[test]
    fn test_convolution_in_effect_chain() {
        // Test convolution as part of a full DSP chain
        let chain_json = json!([
            {
                "type": "eq",
                "bands": [
                    { "frequency": 100.0, "gain": 0.0, "q": 1.0 }
                ]
            },
            {
                "type": "convolution",
                "settings": {
                    "irFilePath": "/ir/room.wav",
                    "wetDryMix": 0.2,
                    "preDelayMs": 5.0,
                    "decay": 1.0
                }
            },
            {
                "type": "limiter",
                "settings": {
                    "thresholdDb": -0.3,
                    "releaseMs": 50.0
                }
            }
        ]);

        // Deserialize the chain
        let effects: Vec<EffectType> =
            serde_json::from_value(chain_json).expect("Should deserialize chain");

        assert_eq!(effects.len(), 3);
        assert!(matches!(effects[0], EffectType::Eq { .. }));
        assert!(matches!(effects[1], EffectType::Convolution { .. }));
        assert!(matches!(effects[2], EffectType::Limiter { .. }));

        // Verify convolution settings
        if let EffectType::Convolution { settings } = &effects[1] {
            assert_eq!(settings.ir_file_path, "/ir/room.wav");
            assert_eq!(settings.wet_dry_mix, 0.2);
        } else {
            panic!("Expected Convolution variant");
        }
    }

    #[test]
    fn test_convolution_parameter_ranges() {
        // Test boundary values for convolution
        let test_cases = vec![
            // Wet/dry mix: 0.0 to 1.0
            (0.0, 0.0, 0.5),   // Fully dry
            (1.0, 0.0, 1.0),   // Fully wet
            // Pre-delay: 0 to 100ms
            (0.5, 0.0, 1.0),   // No pre-delay
            (0.5, 100.0, 1.0), // Max pre-delay
            // Decay: 0.5 to 2.0
            (0.5, 10.0, 0.5),  // Minimum decay
            (0.5, 10.0, 2.0),  // Maximum decay
        ];

        for (wet_dry, pre_delay, decay) in test_cases {
            let data = ConvolutionData {
                ir_file_path: "/test.wav".to_string(),
                wet_dry_mix: wet_dry,
                pre_delay_ms: pre_delay,
                decay,
            };

            let json = serde_json::to_value(&data).expect("Should serialize");
            let restored: ConvolutionData =
                serde_json::from_value(json).expect("Should deserialize");

            assert_eq!(restored.wet_dry_mix, wet_dry);
            assert_eq!(restored.pre_delay_ms, pre_delay);
            assert_eq!(restored.decay, decay);
        }
    }

    #[test]
    fn test_convolution_empty_path() {
        // Test with empty IR file path (valid for unsaved/new convolution)
        let data = ConvolutionData {
            ir_file_path: String::new(),
            wet_dry_mix: 0.5,
            pre_delay_ms: 0.0,
            decay: 1.0,
        };

        let json = serde_json::to_value(&data).expect("Should serialize");
        let restored: ConvolutionData = serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(restored.ir_file_path, "");
    }

    #[test]
    fn test_convolution_various_file_paths() {
        // Test different path formats
        let paths = vec![
            "/absolute/unix/path.wav",
            "C:\\Windows\\path.wav",
            "relative/path.wav",
            "file with spaces.flac",
            "ファイル.wav",  // Unicode path
        ];

        for path in paths {
            let data = ConvolutionData {
                ir_file_path: path.to_string(),
                wet_dry_mix: 0.3,
                pre_delay_ms: 0.0,
                decay: 1.0,
            };

            let json = serde_json::to_value(&data).expect("Should serialize");
            let restored: ConvolutionData =
                serde_json::from_value(json).expect("Should deserialize");

            assert_eq!(restored.ir_file_path, path);
        }
    }
}

// ============================================================================
// 9. RESAMPLING SETTINGS TESTS
// ============================================================================

mod resampling_tests {
    use serde_json::json;

    /// Resampling settings structure (mirrors audio_settings.rs)
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    pub struct ResamplingSettings {
        pub quality: String,
        pub target_rate: u32,
        pub backend: String,
    }

    impl Default for ResamplingSettings {
        fn default() -> Self {
            Self {
                quality: "high".to_string(),
                target_rate: 0,
                backend: "auto".to_string(),
            }
        }
    }

    #[test]
    fn test_resampling_default_values() {
        let settings = ResamplingSettings::default();

        assert_eq!(settings.quality, "high");
        assert_eq!(settings.target_rate, 0);
        assert_eq!(settings.backend, "auto");
    }

    #[test]
    fn test_resampling_quality_values() {
        let valid_qualities = vec!["fast", "balanced", "high", "maximum"];

        for quality in valid_qualities {
            let settings = ResamplingSettings {
                quality: quality.to_string(),
                target_rate: 0,
                backend: "auto".to_string(),
            };

            let json = serde_json::to_value(&settings).expect("Should serialize");
            let restored: ResamplingSettings =
                serde_json::from_value(json).expect("Should deserialize");

            assert_eq!(restored.quality, quality);
        }
    }

    #[test]
    fn test_resampling_target_rate_values() {
        // 0 = auto, otherwise specific rates
        let valid_rates = vec![0, 44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000];

        for rate in valid_rates {
            let settings = ResamplingSettings {
                quality: "high".to_string(),
                target_rate: rate,
                backend: "auto".to_string(),
            };

            let json = serde_json::to_value(&settings).expect("Should serialize");
            let restored: ResamplingSettings =
                serde_json::from_value(json).expect("Should deserialize");

            assert_eq!(restored.target_rate, rate);
        }
    }

    #[test]
    fn test_resampling_backend_values() {
        let valid_backends = vec!["auto", "rubato", "r8brain"];

        for backend in valid_backends {
            let settings = ResamplingSettings {
                quality: "high".to_string(),
                target_rate: 0,
                backend: backend.to_string(),
            };

            let json = serde_json::to_value(&settings).expect("Should serialize");
            let restored: ResamplingSettings =
                serde_json::from_value(json).expect("Should deserialize");

            assert_eq!(restored.backend, backend);
        }
    }

    #[test]
    fn test_resampling_serialization_uses_camelcase() {
        let settings = ResamplingSettings {
            quality: "maximum".to_string(),
            target_rate: 192000,
            backend: "rubato".to_string(),
        };

        let json = serde_json::to_value(&settings).expect("Should serialize");

        // Verify camelCase keys in JSON
        assert!(json.get("quality").is_some());
        assert!(json.get("targetRate").is_some());
        assert!(json.get("backend").is_some());

        // Verify snake_case keys are NOT present
        assert!(json.get("target_rate").is_none());
    }

    #[test]
    fn test_resampling_full_roundtrip() {
        let original = ResamplingSettings {
            quality: "balanced".to_string(),
            target_rate: 96000,
            backend: "rubato".to_string(),
        };

        // Serialize
        let json = serde_json::to_value(&original).expect("Should serialize");

        // Deserialize
        let restored: ResamplingSettings =
            serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(original.quality, restored.quality);
        assert_eq!(original.target_rate, restored.target_rate);
        assert_eq!(original.backend, restored.backend);
    }

    #[test]
    fn test_resampling_json_deserialization() {
        let json = json!({
            "quality": "high",
            "targetRate": 48000,
            "backend": "auto"
        });

        let settings: ResamplingSettings =
            serde_json::from_value(json).expect("Should deserialize");

        assert_eq!(settings.quality, "high");
        assert_eq!(settings.target_rate, 48000);
        assert_eq!(settings.backend, "auto");
    }

    #[test]
    fn test_resampling_persistence_format() {
        // Test the format stored in user_settings table
        let settings = json!({
            "quality": "maximum",
            "target_rate": 384000,
            "backend": "r8brain"
        });

        // Simulate database roundtrip (stored as string)
        let stored = settings.to_string();
        let restored: serde_json::Value =
            serde_json::from_str(&stored).expect("Should parse stored JSON");

        assert_eq!(restored["quality"], "maximum");
        assert_eq!(restored["target_rate"], 384000);
        assert_eq!(restored["backend"], "r8brain");
    }
}
