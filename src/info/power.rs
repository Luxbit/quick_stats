use battery::Manager;
use battery::units::ratio::percent;
use std::collections::HashMap;

pub struct BatteryInfo {
    pub has_battery: bool,
    pub charge_percent: Option<f32>,
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
    };

    for battery in batteries {
        let battery = battery.unwrap();
        battery_info.has_battery = true;
        battery_info.charge_percent = Some(battery.state_of_charge().get::<percent>());
    }

    battery_info
}
