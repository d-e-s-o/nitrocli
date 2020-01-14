// Copyright (C) 2020 Robin Krahl <robin.krahl@ireas.org>
// SPDX-License-Identifier: CC0-1.0

//! Enumerates all connected Nitrokey devices and prints some information about them.

use nitrokey::Device as _;

fn main() -> Result<(), nitrokey::Error> {
    let mut manager = nitrokey::take()?;
    let device_infos = nitrokey::list_devices()?;
    if device_infos.is_empty() {
        println!("No Nitrokey device found");
    } else {
        println!("path\t\tmodel\tfirmware version\tserial number");
        for device_info in device_infos {
            let device = manager.connect_path(device_info.path.clone())?;
            let model = device.get_model();
            let status = device.get_status()?;
            println!(
                "{}\t{}\t{}\t\t\t{:08x}",
                device_info.path, model, status.firmware_version, status.serial_number
            );
        }
    }
    Ok(())
}
