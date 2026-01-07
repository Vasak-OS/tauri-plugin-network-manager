use zbus::zvariant::Value;
use zbus::names::InterfaceName;
use crate::error::Result;
use crate::models::WiFiSecurityType;
use crate::nm_constants::*;

/// Helper functions for NetworkManager operations
pub struct NetworkManagerHelpers;

impl NetworkManagerHelpers {
    /// Detect WiFi security type from access point properties
    ///
    /// This function checks the access point's security flags and key management
    /// to determine the security type (None, WEP, WPA-PSK, WPA2-PSK, WPA3-PSK, WPA-EAP)
    pub fn detect_security_type(
        ap_props: &zbus::blocking::fdo::PropertiesProxy,
    ) -> Result<WiFiSecurityType> {
        // Get security flags
        let flags_variant = ap_props.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM_ACCESS_POINT),
            "Flags",
        )?;
        let wpa_flags_variant = ap_props.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM_ACCESS_POINT),
            "WpaFlags",
        )?;
        let rsn_flags_variant = ap_props.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM_ACCESS_POINT),
            "RsnFlags",
        )?;

        let flags = if let Some(Value::U32(f)) = flags_variant.downcast_ref() { *f } else { 0 };
        let wpa = if let Some(Value::U32(w)) = wpa_flags_variant.downcast_ref() { *w } else { 0 };
        let rsn = if let Some(Value::U32(r)) = rsn_flags_variant.downcast_ref() { *r } else { 0 };

        // Try to get key-mgmt first (more accurate)
        if let Ok(key_mgmt_variant) = ap_props.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM_ACCESS_POINT),
            "KeyMgmt",
        ) {
            if let Some(Value::Str(key_mgmt)) = key_mgmt_variant.downcast_ref() {
                return Ok(match key_mgmt.as_str() {
                    "none" => WiFiSecurityType::None,
                    "wpa-psk" => WiFiSecurityType::WpaPsk,
                    "wpa-eap" => WiFiSecurityType::WpaEap,
                    "sae" => WiFiSecurityType::Wpa3Psk,
                    _ => WiFiSecurityType::None,
                });
            }
        }

        // Fallback to flags-based detection
        Ok(if flags & SECURITY_FLAG_NONE != 0 {
            WiFiSecurityType::None
        } else if flags & SECURITY_FLAG_WEP != 0 {
            WiFiSecurityType::Wep
        } else if wpa != 0 && rsn == 0 {
            WiFiSecurityType::WpaPsk
        } else if rsn != 0 {
            if wpa != 0 {
                WiFiSecurityType::Wpa2Psk
            } else {
                WiFiSecurityType::Wpa3Psk
            }
        } else {
            WiFiSecurityType::None
        })
    }

    /// Parse SSID from D-Bus byte array
    pub fn parse_ssid(ssid_variant: &zbus::zvariant::OwnedValue) -> String {
        match ssid_variant.downcast_ref() {
            Some(Value::Array(ssid_bytes)) => {
                let bytes: Vec<u8> = ssid_bytes
                    .iter()
                    .filter_map(|v| {
                        if let Value::U8(b) = v {
                            Some(*b)
                        } else {
                            None
                        }
                    })
                    .collect();
                String::from_utf8_lossy(&bytes).to_string()
            }
            _ => "Unknown".to_string(),
        }
    }

    /// Get connectivity state from NetworkManager
    pub fn has_internet_connectivity(
        proxy: &zbus::blocking::fdo::PropertiesProxy,
    ) -> Result<bool> {
        let connectivity_variant = proxy.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM),
            "Connectivity",
        )?;

        Ok(match connectivity_variant.downcast_ref() {
            Some(Value::U32(CONNECTIVITY_FULL)) => true,
            _ => false,
        })
    }
}
