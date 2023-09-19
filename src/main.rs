mod heart_rate;
mod hrv;

use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::error::Error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time;
use uuid::uuid;

async fn find_matching_peripheral_by_local_name(central: &Adapter, pattern: &str) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains(pattern))
        {
            return Some(p);
        }
    }
    None
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    // scan
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();
    central.start_scan(ScanFilter::default()).await?;
    time::sleep(Duration::from_secs(5)).await;
    // find + connect + discover
    let peripheral = find_matching_peripheral_by_local_name(&central, "MYZONE").await;
    if peripheral.is_none() {
        panic!("failed to find peripheral");
    }
    let peripheral = peripheral.unwrap();
    peripheral.connect().await?;
    peripheral.discover_services().await?;
    // extract services
    let services = peripheral.services();
    let battery_service = services.iter().find(|s| s.uuid == uuid!("0000180a-0000-1000-8000-00805f9b34fb")).unwrap();
    let heart_rate_service = services.iter().find(|s| s.uuid == uuid!("0000180d-0000-1000-8000-00805f9b34fb")).unwrap();
    let myzone_service = services.iter().find(|s| s.uuid == uuid!("d924e000-4664-96f6-e88d-ea30afe35a90")).unwrap();
    // extract characteristics
    let characteristics = peripheral.characteristics();
    // device info characteristics
    let battery_level_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a19-0000-1000-8000-00805f9b34fb")).unwrap();
    let system_id_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a23-0000-1000-8000-00805f9b34fb")).unwrap();
    let model_number_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a24-0000-1000-8000-00805f9b34fb")).unwrap();
    let serial_number_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a25-0000-1000-8000-00805f9b34fb")).unwrap();
    let firmware_revision_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a26-0000-1000-8000-00805f9b34fb")).unwrap();
    let hardware_revision_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a27-0000-1000-8000-00805f9b34fb")).unwrap();
    let software_revision_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a28-0000-1000-8000-00805f9b34fb")).unwrap();
    let manufacturer_name_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a29-0000-1000-8000-00805f9b34fb")).unwrap();
    // standard characteristics
    let heart_rate_measurement_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a37-0000-1000-8000-00805f9b34fb")).unwrap();
    let body_sensor_location_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("00002a38-0000-1000-8000-00805f9b34fb")).unwrap();
    // non-standard characteristics
    let send_message_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("d924e001-4664-96f6-e88d-ea30afe35a90")).unwrap();
    let receive_message_response_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("d924e002-4664-96f6-e88d-ea30afe35a90")).unwrap();
    let burst_data_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("d924e003-4664-96f6-e88d-ea30afe35a90")).unwrap();
    let receive_heart_rate_with_rir_flag_characteristic = characteristics.iter().find(|c| c.uuid == uuid!("d924e004-4664-96f6-e88d-ea30afe35a90")).unwrap();
    // subscribe?
    peripheral.subscribe(heart_rate_measurement_characteristic).await?;
    // read?
    let mut heart_rates = vec![];
    let start = std::time::Instant::now();
    let mut last_blank_read = None;
    loop {
        //let result = peripheral.read(receive_heart_rate_with_rir_flag_characteristic).await?; // TODO: figure out this nonstanrd format (example: 005a)
        let result = peripheral.read(heart_rate_measurement_characteristic).await?; // TODO: is this entire message parsed correctly?
        // TODO: sleep or no?
        let heart_rate = heart_rate::HeartRate::new(&result).unwrap();
        let bpm = heart_rate.bpm();
        // make sure rr is stable
        if heart_rate.rr().is_none() {
            println!("skipping heart rate reading with no R-R inteval");
            last_blank_read = Some(std::time::Instant::now());
            continue;
        }
        let elapsed = start.elapsed().as_secs();
        let stabilized = elapsed > 5;
        if stabilized == false {
            println!("not stabilized yet; skipping first 5 seconds of data");
            continue;
        }
        if last_blank_read.is_some() {
            let elapsed = last_blank_read.unwrap().elapsed().as_secs();
            let stabilized = elapsed > 3;
            if stabilized == false {
                println!("not stabilized yet; skipping 3 seconds of data");
                continue;
            }
        }
        // push
        heart_rates.push(heart_rate);
        // calculate hrv
        let rr_intervals: Vec<u16> = heart_rates.iter()
            .filter_map(|hr| hr.rr().as_ref())
            .flatten()      
            .cloned()        
            .collect();
        let (sdnn_hrv, rmssd_hrv) = hrv::calculate_hrv(&rr_intervals);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
        // print
        println!("{now},{bpm},{sdnn_hrv},{rmssd_hrv},{}", hex::encode(&result));
    }
    Ok(())
}
