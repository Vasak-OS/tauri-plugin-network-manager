use zbus::zvariant::Value;
use zbus::names::InterfaceName;
use crate::error::Result;
use crate::models::WiFiSecurityType;
use crate::nm_constants::*;

pub struct NetworkManagerHelpers;

impl NetworkManagerHelpers {
    /// Detect WiFi security type from access point properties
    pub fn detect_security_type(
        ap_props: &zbus::blocking::fdo::PropertiesProxy,
    ) -> Result<WiFiSecurityType> {
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

        let flags = if let Ok(Value::U32(f)) = flags_variant.downcast_ref() { f } else { 0 };
        let wpa = if let Ok(Value::U32(w)) = wpa_flags_variant.downcast_ref() { w } else { 0 };
        let rsn = if let Ok(Value::U32(r)) = rsn_flags_variant.downcast_ref() { r } else { 0 };

        if let Ok(key_mgmt_variant) = ap_props.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM_ACCESS_POINT),
            "KeyMgmt",
        ) {
            if let Ok(Value::Str(key_mgmt)) = key_mgmt_variant.downcast_ref() {
                return Ok(match key_mgmt.as_str() {
                    "none" => WiFiSecurityType::None,
                    "wpa-psk" => WiFiSecurityType::WpaPsk,
                    "wpa-eap" => WiFiSecurityType::WpaEap,
                    "sae" => WiFiSecurityType::Wpa3Psk,
                    _ => WiFiSecurityType::None,
                });
            }
        }

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

    pub fn has_internet_connectivity(
        proxy: &zbus::blocking::fdo::PropertiesProxy,
    ) -> Result<bool> {
        let connectivity_variant = proxy.get(
            InterfaceName::from_static_str_unchecked(IFACE_NM),
            "Connectivity",
        )?;

        Ok(match connectivity_variant.downcast_ref() {
            Ok(Value::U32(CONNECTIVITY_FULL)) => true,
            _ => false,
        })
    }

    pub fn ssid_from_value(value: &Value<'_>) -> String {
        match value {
            Value::Array(ssid_bytes) => {
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
}
