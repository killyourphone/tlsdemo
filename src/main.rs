use std::{
    ptr::null_mut,
    sync::Arc,
    thread::{self, spawn},
    time::Duration,
};

use embedded_svc::wifi::{
    ClientConfiguration, ClientConnectionStatus, ClientIpStatus, ClientStatus, Configuration,
    Status, Wifi,
};
use esp_idf_svc::{
    netif::EspNetifStack, nvs::EspDefaultNvs, sysloop::EspSysLoopStack, wifi::EspWifi,
};
use esp_idf_sys::c_types::c_uint;
#[allow(unused)]
use esp_idf_sys::{self as _, timeval, timezone};

const SSID: &str = todo!();
const PASS: &str = todo!();

#[allow(dead_code)]
fn wifi(
    netif_stack: Arc<EspNetifStack>,
    sys_loop_stack: Arc<EspSysLoopStack>,
    default_nvs: Arc<EspDefaultNvs>,
) {
    let mut wifi = EspWifi::new(netif_stack, sys_loop_stack, default_nvs).expect("Need wifi");

    unsafe {
        esp_idf_sys::esp_wifi_set_ps(esp_idf_sys::wifi_ps_type_t_WIFI_PS_NONE);
    }

    println!("Wifi created, about to scan");

    let ap_infos = wifi.scan().expect("Need scan results");

    let ours = ap_infos.into_iter().find(|a| a.ssid == SSID);

    let channel = if let Some(ours) = ours {
        println!(
            "Found configured access point {} on channel {}",
            SSID, ours.channel
        );
        Some(ours.channel)
    } else {
        println!(
            "Configured access point {} not found during scanning, will go with unknown channel",
            SSID
        );
        None
    };

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASS.into(),
        channel,
        ..Default::default()
    }))
    .expect("Couldn't set configuration");

    println!("Wifi configuration set, about to get status");

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(_))),
        _,
    ) = status
    {
        println!("Wifi connected");

        thread::Builder::new()
            .stack_size(32768)
            .spawn(move || {
                unsafe {
                    esp_idf_sys::vTaskPrioritySet(null_mut(), 1u32 as c_uint);
                }
                loop {
                    match attohttpc::get("https://www.espressif.com/robots.txt").send() {
                        Ok(response) => {
                            println!("Text of page is {}", response.text().unwrap());
                            return;
                        }
                        Err(err) => {
                            println!("Couldn't fetch page, will sleep: {}", err.to_string());
                            println!("Maybe check your system time if you have certificate validation issues?");
                            // unsafe {
                            //     esp_idf_sys::settimeofday(
                            //         &timeval {
                            //             tv_sec: todo!(),
                            //             tv_usec: 0,
                            //         },
                            //         &timezone {
                            //             tz_minuteswest: 0,
                            //             tz_dsttime: 0,
                            //         },
                            //     );
                            // }
                        }
                    }
                    thread::sleep(Duration::from_secs(5));
                }
            })
            .expect("Failed to spawn thread");

        loop {
            // Trap execution here
            thread::sleep(Duration::from_millis(1000));
        }
    } else {
        println!("Unexpected Wifi status: {:?}", status);
        panic!();
    }
}

fn main() {
    // Temporary. Will disappear once ESP-IDF 4.4 is released, but for now it is necessary to call this function once,
    // or else some patches to the runtime implemented by esp-idf-sys might not link properly.
    esp_idf_sys::link_patches();
    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let netif_stack = Arc::new(EspNetifStack::new().expect("Need netif stack"));
    let sys_loop_stack = Arc::new(EspSysLoopStack::new().expect("Need sys loop stack"));
    let default_nvs = Arc::new(EspDefaultNvs::new().expect("Need default nvs"));

    wifi(netif_stack, sys_loop_stack, default_nvs);
}
