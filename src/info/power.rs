use battery::units::{ratio::percent, energy::watt_hour};
use battery::Manager;
use std::collections::HashMap;

pub struct BatteryInfo {
    pub has_battery: bool,
    pub charge_percent: Option<f32>,
    pub is_charging: Option<bool>,
    pub wh_capacity: Option<f32>
}

pub fn get_battery_info() -> BatteryInfo {
    // Create a new battery manager
    let manager = Manager::new().unwrap();

    // Get the list of batteries
    let batteries = manager.batteries().unwrap();

    // Collect battery information
    let mut battery_info = BatteryInfo {
        has_battery: false,
        charge_percent: None,
        is_charging: None,
        wh_capacity: None,
    };

    for battery in batteries {
        let battery = battery.unwrap();
        battery_info.has_battery = true;
        battery_info.charge_percent = Some(battery.state_of_charge().get::<percent>());
        battery_info.is_charging = Some(battery.state() == battery::State::Charging);
        battery_info.wh_capacity = Some(battery.energy_full_design().get::<watt_hour>());
    }

    battery_info
}
