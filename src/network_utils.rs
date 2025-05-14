pub fn get_wifi_icon(strength: u8) -> String {
    match strength {
        0..=20 => "wifi-signal-weak".to_string(),
        21..=40 => "wifi-signal-low".to_string(),
        41..=60 => "wifi-signal-medium".to_string(),
        61..=80 => "wifi-signal-good".to_string(),
        81..=100 => "wifi-signal-excellent".to_string(),
        _ => "wifi-signal-none".to_string(),
    }
}
