//! Configuration integration tests
//!
//! Tests configuration persistence, validation, and hot reload scenarios.

use surface_dial::config::Config;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_roundtrip() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    // Create and modify config
    let mut config = Config::default();
    config.volume.step_min = 1;
    config.volume.step_max = 15;
    config.microphone.mode_duration = 20;
    config.media_control.enabled = true;

    // Save
    config.save_to(&config_path).unwrap();

    // Load fresh
    let loaded = Config::load_from(&config_path).unwrap();

    // Verify all values persisted
    assert_eq!(loaded.volume.step_min, 1);
    assert_eq!(loaded.volume.step_max, 15);
    assert_eq!(loaded.microphone.mode_duration, 20);
    assert!(loaded.media_control.enabled);
}

#[test]
fn test_config_partial_file() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    // Write only partial config
    fs::write(
        &config_path,
        r#"
[volume]
step_min = 3
step_max = 12
"#,
    )
    .unwrap();

    // Load should fill in defaults for missing sections
    let loaded = Config::load_from(&config_path).unwrap();

    // Specified values
    assert_eq!(loaded.volume.step_min, 3);
    assert_eq!(loaded.volume.step_max, 12);

    // Default values for unspecified sections
    let default_config = Config::default();
    assert_eq!(loaded.microphone.mode_duration, default_config.microphone.mode_duration);
    assert_eq!(loaded.media_control.enabled, default_config.media_control.enabled);
}

#[test]
fn test_config_unknown_keys_ignored() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    // Write config with unknown keys
    fs::write(
        &config_path,
        r#"
[volume]
step_min = 2
step_max = 8
unknown_key = "should be ignored"

[unknown_section]
foo = "bar"
"#,
    )
    .unwrap();

    // Should load without error
    let loaded = Config::load_from(&config_path).unwrap();
    assert_eq!(loaded.volume.step_min, 2);
    assert_eq!(loaded.volume.step_max, 8);
}

#[test]
fn test_config_get_value() {
    let config = Config::default();

    // Test various key paths
    let vol_min = config.get_value("volume.step_min").unwrap();
    assert_eq!(vol_min.as_i64().unwrap(), 2);

    let curve = config.get_value("volume.curve").unwrap();
    assert!(curve.as_str().is_some());

    // media_control.enabled is a valid key
    let media_enabled = config.get_value("media_control.enabled").unwrap();
    assert!(media_enabled.as_bool().is_some());
}

#[test]
fn test_config_get_value_invalid_key() {
    let config = Config::default();

    let result = config.get_value("nonexistent.key");
    assert!(result.is_err());

    let result = config.get_value("volume.nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_config_set_value() {
    let mut config = Config::default();

    // Set various types
    config.set_value("volume.step_min", "5").unwrap();
    assert_eq!(config.volume.step_min, 5);

    config.set_value("volume.step_max", "15").unwrap();
    assert_eq!(config.volume.step_max, 15);

    config.set_value("media_control.enabled", "true").unwrap();
    assert!(config.media_control.enabled);

    // For curve, just pass the value without quotes
    config.set_value("volume.curve", "exponential").unwrap();
    assert_eq!(config.volume.curve, "exponential");
}

#[test]
fn test_config_set_value_invalid() {
    let mut config = Config::default();

    // Invalid key
    let result = config.set_value("nonexistent.key", "5");
    assert!(result.is_err());

    // Invalid value type
    let result = config.set_value("volume.step_min", "not_a_number");
    assert!(result.is_err());
}

#[test]
fn test_config_reset_section() {
    let mut config = Config::default();

    // Modify volume section
    config.volume.step_min = 10;
    config.volume.step_max = 18;
    config.volume.curve = "custom".to_string();

    // Reset just volume section
    config.reset_section("volume").unwrap();

    // Volume should be defaults
    let default_config = Config::default();
    assert_eq!(config.volume.step_min, default_config.volume.step_min);
    assert_eq!(config.volume.step_max, default_config.volume.step_max);
    assert_eq!(config.volume.curve, default_config.volume.curve);
}

#[test]
fn test_config_reset_invalid_section() {
    let mut config = Config::default();

    let result = config.reset_section("nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_config_validation_on_set() {
    let mut config = Config::default();

    // Valid range (step_min/max are 1-20)
    assert!(config.set_value("volume.step_min", "1").is_ok());
    assert!(config.set_value("volume.step_max", "20").is_ok());

    // Out of range should fail
    assert!(config.set_value("volume.step_min", "0").is_err());
    assert!(config.set_value("volume.step_max", "100").is_err());

    // Boolean parsing
    assert!(config.set_value("media_control.enabled", "true").is_ok());
    assert!(config.set_value("media_control.enabled", "false").is_ok());
}

#[test]
fn test_config_file_not_found_behavior() {
    let temp = TempDir::new().unwrap();
    let nonexistent = temp.path().join("does_not_exist.toml");

    // load_from returns error for non-existent file
    let result = Config::load_from(&nonexistent);
    assert!(result.is_err());

    // But Config::load() (without path) returns defaults
    // (uses Config::load_from().unwrap_or_default() internally)
}

#[test]
fn test_config_malformed_toml() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    // Write malformed TOML
    fs::write(
        &config_path,
        r#"
[volume
step_min = 2
"#,
    )
    .unwrap();

    // Should error on malformed TOML
    let result = Config::load_from(&config_path);
    assert!(result.is_err());
}

#[test]
fn test_config_serialization_format() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    let config = Config::default();
    config.save_to(&config_path).unwrap();

    let content = fs::read_to_string(&config_path).unwrap();

    // Verify TOML structure
    assert!(content.contains("[volume]"));
    assert!(content.contains("[microphone]"));
    assert!(content.contains("step_min"));
    assert!(content.contains("step_max"));
}

#[test]
fn test_config_multiple_saves() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    let mut config = Config::default();

    // Multiple save cycles
    for i in 1..=5 {
        config.volume.step_min = i;
        config.save_to(&config_path).unwrap();

        let loaded = Config::load_from(&config_path).unwrap();
        assert_eq!(loaded.volume.step_min, i);
    }
}

#[test]
fn test_config_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("config.toml");

    // Create initial config
    let config = Config::default();
    config.save_to(&config_path).unwrap();

    let config_path = Arc::new(config_path);

    // Spawn readers
    let mut handles = Vec::new();
    for _ in 0..4 {
        let path = Arc::clone(&config_path);
        handles.push(thread::spawn(move || {
            for _ in 0..10 {
                let _ = Config::load_from(path.as_path());
            }
        }));
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn test_config_curve_values() {
    let mut config = Config::default();

    // Valid curve values
    assert!(config.set_value("volume.curve", "linear").is_ok());
    assert_eq!(config.volume.curve, "linear");

    assert!(config.set_value("volume.curve", "logarithmic").is_ok());
    assert_eq!(config.volume.curve, "logarithmic");

    assert!(config.set_value("volume.curve", "exponential").is_ok());
    assert_eq!(config.volume.curve, "exponential");

    assert!(config.set_value("volume.curve", "custom").is_ok());
    assert_eq!(config.volume.curve, "custom");

    // Invalid curve value
    assert!(config.set_value("volume.curve", "invalid").is_err());
}

#[test]
fn test_config_validate_cross_field() {
    let mut config = Config::default();

    // Set step_min > step_max (invalid)
    config.volume.step_min = 15;
    config.volume.step_max = 5;

    let errors = config.validate();
    assert!(!errors.is_empty());
    assert!(errors[0].contains("step_min"));
}
