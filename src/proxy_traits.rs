use std::collections::HashMap;
use zbus::{names::InterfaceName, zvariant::{OwnedValue, Value}};
use crate::models::*;

impl<'a> DeviceProxy for zbus::blocking::Proxy<'a> {
    fn connection(&self) -> &zbus::blocking::Connection {
        self.connection()
    }
    
    fn destination(&self) -> &str {
        zbus::blocking::Proxy::destination(self)
    }
    
    fn path(&self) -> &str {
        zbus::blocking::Proxy::path(self)
    }

    fn device_type(&self) -> zbus::Result<u32> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.Device")?
            .build()?;
        
        let device_type_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device"), "DeviceType")?;
        
        match device_type_variant.downcast_ref() {
            Some(zbus::zvariant::Value::U32(device_type)) => Ok(*device_type),
            _ => Err(zbus::Error::from(std::io::Error::new(std::io::ErrorKind::Other, "Failed to parse device type"))),
        }
    }
}

impl WirelessDeviceProxy for zbus::blocking::Proxy<'_> {
    fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.Device.Wireless")?
            .build()?;
        
        let aps_variant: OwnedValue = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.Device.Wireless"), "AccessPoints")?.try_into()?;
        
        match aps_variant.downcast_ref() {
            Some(Value::Array(arr)) => Ok(arr.into_iter()
                .filter_map(|v| match v {
                    zbus::zvariant::Value::ObjectPath(path) => Some(zbus::zvariant::OwnedObjectPath::from(path.to_owned())),
                    _ => None,
                })
                .collect()),
            _ => Err(zbus::Error::Failure("Failed to parse access points".into())),
        }
    }
}

impl AccessPointProxy for zbus::blocking::Proxy<'_> {
    fn ssid(&self) -> zbus::Result<Vec<u8>> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.AccessPoint")?
            .build()?;
        
        let ssid_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"), &InterfaceName::from_static_str_unchecked("Ssid"))?;
        
        match ssid_variant.downcast_ref() {
            Some(zbus::zvariant::Value::Array(v)) => {
                let bytes: Vec<u8> = v.iter()
                    .filter_map(|val| match val {
                        zbus::zvariant::Value::U8(byte) => Some(*byte),
                        _ => None,
                    })
                    .collect();
                Ok(bytes)
            },
            _ => Err(zbus::Error::Failure("Failed to parse SSID".into())),
        }
    }
    
    fn strength(&self) -> zbus::Result<u8> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.AccessPoint")?
            .build()?;
        
        let strength_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"), &InterfaceName::from_static_str_unchecked("Strength"))?;
        
        match strength_variant.downcast_ref() {
            Some(zbus::zvariant::Value::U8(v)) => Ok(*v),
            _ => Err(zbus::Error::Failure("Failed to parse strength".into())),
        }
    }
    
    fn security_type(&self) -> zbus::Result<WiFiSecurityType> {
        let properties_proxy = zbus::blocking::fdo::PropertiesProxy::builder(self.connection())
            .destination(self.destination())?
            .path(self.path())?
            .interface("org.freedesktop.NetworkManager.AccessPoint")?
            .build()?;
        
        let wpa_flags_variant = properties_proxy.get(InterfaceName::from_static_str_unchecked("org.freedesktop.NetworkManager.AccessPoint"), &InterfaceName::from_static_str_unchecked("WpaFlags"))?;
        
        let wpa_flags: u32 = match wpa_flags_variant.downcast_ref() {
            Some(zbus::zvariant::Value::U32(v)) => Ok(*v),
            _ => Err(zbus::Error::Failure("Failed to parse WPA flags".into())),
        }?;
        
        Ok(match wpa_flags {
            0 => WiFiSecurityType::None,
            _ => WiFiSecurityType::WpaPsk,
        })
    }
}

impl ConnectionProxy for zbus::blocking::Proxy<'_> {
    fn add_connection(&self, _settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
        // TODO: Implement actual connection addition logic
        Err(zbus::Error::Failure("Connection addition not implemented".into()))
    }
    
    fn call_add_connection(&self, _settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath> {
        // TODO: Implement actual connection addition logic
        Err(zbus::Error::Failure("Connection addition not implemented".into()))
    }
}
