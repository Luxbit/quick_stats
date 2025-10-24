use battery::units::{ratio::percent, energy::watt_hour};
use battery::Manager;
use std::collections::HashMap;

pub struct BatteryInfo {
    pub has_battery: bool,
    pub charge_percent: Option<f32>,
    pub is_charging: Option<bool>,
    pub on_ac_power: Option<bool>,
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
        on_ac_power: None,
        wh_capacity: None,
    };

    for battery in batteries {
        let battery = battery.unwrap();
        battery_info.has_battery = true;
        battery_info.charge_percent = Some(battery.state_of_charge().get::<percent>());

        let state = battery.state();
        battery_info.is_charging = Some(state == battery::State::Charging);

        // Detect AC power: if state is NOT Discharging, we're likely on AC
        // This covers Charging, Full, and Unknown states which all indicate AC connection
        // or at least not actively draining the battery
        let on_ac = match state {
            battery::State::Discharging | battery::State::Empty => false,
            battery::State::Charging | battery::State::Full | battery::State::Unknown | _ => true,
        };
        battery_info.on_ac_power = Some(on_ac);

        battery_info.wh_capacity = Some(battery.energy_full_design().get::<watt_hour>());
    }

    battery_info
}
