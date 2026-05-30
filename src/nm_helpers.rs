use zbus::zvariant::Value;
use zbus::names::InterfaceName;
use crate::error::Result;
use crate::models::WiFiSecurityType;
use crate::nm_constants::*;

pub struct NetworkManagerHelpers;

impl NetworkManagerHelpers {
    /// Detect WiFi security type from access point properties.
    ///
    /// Uses the following NM D-Bus properties in order of reliability:
    /// 1. `KeyMgmt` — string property (most accurate, but may not be available on older NM)
    /// 2. `WpaFlags` / `RsnFlags` — checks KEY_MGMT bits for PSK, 802.1X, SAE
    /// 3. `Flags` — fallback: if no WPA/RSN flags but privacy bit set → WEP
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

        // 1. Use KeyMgmt string property if available (most reliable)
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

        // 2. Check for open network
        if flags == AP_FLAGS_NONE {
            return Ok(WiFiSecurityType::None);
        }

        // 3. Check key management bits in security flags
        if rsn & SEC_FLAGS_KEY_MGMT_802_1X != 0 || wpa & SEC_FLAGS_KEY_MGMT_802_1X != 0 {
            return Ok(WiFiSecurityType::WpaEap);
        }

        if rsn & SEC_FLAGS_KEY_MGMT_SAE != 0 {
            return Ok(WiFiSecurityType::Wpa3Psk);
        }

        if rsn & SEC_FLAGS_KEY_MGMT_PSK != 0 {
            return Ok(WiFiSecurityType::Wpa2Psk);
        }

        if wpa & SEC_FLAGS_KEY_MGMT_PSK != 0 {
            return Ok(WiFiSecurityType::WpaPsk);
        }

        // 4. Fallback: presence of encryption flags without key management bits
        if rsn != 0 {
            return if wpa != 0 {
                Ok(WiFiSecurityType::Wpa2Psk)
            } else {
                Ok(WiFiSecurityType::Wpa3Psk)
            };
        }

        if wpa != 0 {
            return Ok(WiFiSecurityType::WpaPsk);
        }

        // 5. Privacy flag set but no WPA/RSN → likely WEP
        if flags & AP_FLAGS_PRIVACY != 0 {
            return Ok(WiFiSecurityType::Wep);
        }

        Ok(WiFiSecurityType::None)
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
