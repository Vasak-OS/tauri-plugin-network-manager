use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use zbus::{names::InterfaceName, zvariant::{OwnedValue, Value}};

pub trait WirelessDeviceProxy {
  fn get_access_points(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

pub trait DeviceProxy {
  fn connection(&self) -> &zbus::blocking::Connection;
  fn destination(&self) -> &str;
  fn path(&self) -> &str;
  fn device_type(&self) -> zbus::Result<u32>;
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

pub trait AccessPointProxy {
  fn ssid(&self) -> zbus::Result<Vec<u8>>;
  fn strength(&self) -> zbus::Result<u8>;
  fn security_type(&self) -> zbus::Result<WiFiSecurityType>;
}

pub trait ConnectionProxy {
  fn add_connection(&self, settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
  fn call_add_connection(&self, settings: &HashMap<String, HashMap<String, String>>) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub name: String,
    pub ssid: String,
    pub connection_type: String,
    pub icon: String,
    pub ip_address: String,
    pub mac_address: String,
    pub signal_strength: u8,
    pub security_type: WiFiSecurityType,
    pub is_connected: bool,
}

impl Default for NetworkInfo {
    fn default() -> Self {
        Self {
            name: String::from("Unknown"),
            ssid: String::from("Unknown"),
            connection_type: String::from("Unknown"),
            icon: String::from("network-offline-symbolic"), // icono por defecto
            ip_address: String::from("0.0.0.0"),
            mac_address: String::from("00:00:00:00:00:00"),
            signal_strength: 0,
            security_type: WiFiSecurityType::None,
            is_connected: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum WiFiSecurityType {
    None,
    Wep,
    WpaPsk,
    WpaEap,
    Wpa2Psk,
    Wpa3Psk,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WiFiNetwork {
    pub ssid: String,
    pub signal_strength: u8,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WiFiConnectionConfig {
    pub ssid: String,
    pub password: Option<String>,
    pub security_type: WiFiSecurityType,
    pub username: Option<String>,
}

// Removed duplicate init function

#[derive(Clone, Debug)]
pub struct VSKNetworkManager<'a, R: Runtime> {
    pub connection: zbus::blocking::Connection,
    pub proxy: zbus::blocking::fdo::PropertiesProxy<'a>,
    pub app: AppHandle<R>,
}
