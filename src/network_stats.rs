use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::error::Result;
use crate::models::{NetworkStats, BandwidthPoint};

/// Network statistics tracker
pub struct NetworkStatsTracker {
    interface: String,
    last_rx_bytes: u64,
    last_tx_bytes: u64,
    last_check_time: u64,
    start_rx_bytes: u64,
    start_tx_bytes: u64,
    start_time: u64,
}

impl NetworkStatsTracker {
    /// Create a new network stats tracker for the given interface
    pub fn new(interface: String) -> Result<Self> {
        let (rx_bytes, tx_bytes) = Self::read_interface_stats(&interface)?;
        let now = Self::current_timestamp();
        
        Ok(Self {
            interface,
            last_rx_bytes: rx_bytes,
            last_tx_bytes: tx_bytes,
            last_check_time: now,
            start_rx_bytes: rx_bytes,
            start_tx_bytes: tx_bytes,
            start_time: now,
        })
    }
    
    /// Get current network statistics
    pub fn get_stats(&mut self) -> Result<NetworkStats> {
        let (rx_bytes, tx_bytes) = Self::read_interface_stats(&self.interface)?;
        let now = Self::current_timestamp();
        
        // Calculate time delta in seconds
        let time_delta = (now - self.last_check_time) as f64;
        
        // Calculate speeds (bytes per second)
        let download_speed = if time_delta > 0.0 {
            ((rx_bytes - self.last_rx_bytes) as f64 / time_delta) as u64
        } else {
            0
        };
        
        let upload_speed = if time_delta > 0.0 {
            ((tx_bytes - self.last_tx_bytes) as f64 / time_delta) as u64
        } else {
            0
        };
        
        // Update last values
        self.last_rx_bytes = rx_bytes;
        self.last_tx_bytes = tx_bytes;
        self.last_check_time = now;
        
        Ok(NetworkStats {
            download_speed,
            upload_speed,
            total_downloaded: rx_bytes - self.start_rx_bytes,
            total_uploaded: tx_bytes - self.start_tx_bytes,
            connection_duration: now - self.start_time,
            interface: self.interface.clone(),
        })
    }
    
    /// Get bandwidth point for historical tracking
    pub fn get_bandwidth_point(&mut self) -> Result<BandwidthPoint> {
        let stats = self.get_stats()?;
        
        Ok(BandwidthPoint {
            timestamp: Self::current_timestamp(),
            download_speed: stats.download_speed,
            upload_speed: stats.upload_speed,
        })
    }
    
    /// Read interface statistics from /sys/class/net
    fn read_interface_stats(interface: &str) -> Result<(u64, u64)> {
        let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", interface);
        let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", interface);
        
        let rx_bytes = fs::read_to_string(&rx_path)
            .map_err(|e| crate::error::NetworkError::OperationError(
                format!("Failed to read rx_bytes for {}: {}", interface, e)
            ))?
            .trim()
            .parse::<u64>()
            .map_err(|e| crate::error::NetworkError::OperationError(
                format!("Failed to parse rx_bytes: {}", e)
            ))?;
        
        let tx_bytes = fs::read_to_string(&tx_path)
            .map_err(|e| crate::error::NetworkError::OperationError(
                format!("Failed to read tx_bytes for {}: {}", interface, e)
            ))?
            .trim()
            .parse::<u64>()
            .map_err(|e| crate::error::NetworkError::OperationError(
                format!("Failed to parse tx_bytes: {}", e)
            ))?;
        
        Ok((rx_bytes, tx_bytes))
    }
    
    /// Get current timestamp in seconds
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }
}

/// Get list of available network interfaces
pub fn get_network_interfaces() -> Result<Vec<String>> {
    let net_path = "/sys/class/net";
    let entries = fs::read_dir(net_path)
        .map_err(|e| crate::error::NetworkError::OperationError(
            format!("Failed to read {}: {}", net_path, e)
        ))?;
    
    let mut interfaces = Vec::new();
    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(name) = entry.file_name().to_str() {
                // Skip loopback interface
                if name != "lo" {
                    interfaces.push(name.to_string());
                }
            }
        }
    }
    
    Ok(interfaces)
}
