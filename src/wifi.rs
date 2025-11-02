use std::fmt::Write;

use esp_idf_hal::{delay::FreeRtos, modem::Modem};
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi, PmfConfiguration, ScanMethod}};




pub struct Wifi;

impl Wifi {
    pub fn init(modem : Modem) -> EspWifi<'static> {
        let mut wifi_driver = EspWifi::new(
            modem,
            EspSystemEventLoop::take().expect("Failed to take system event loop"),
            Some(EspDefaultNvsPartition::take().expect("Failed to take default nvs partition")),
        )
        .expect("Failed to create esp wifi device");

        let wifi_ssid: String = env!("WIFI_SSID").into();
        let mut wifi_ssid_32: heapless::String<32> = heapless::String::new();
        wifi_ssid_32.write_str(wifi_ssid.as_str()).expect("Failed to parse String to heapless String");

        let wifi_pwd: String = env!("WIFI_PWD").into();
        let mut wifi_pwd_64: heapless::String<64> = heapless::String::new();
        wifi_pwd_64.write_str(wifi_pwd.as_str()).expect("Failed to parse String to heapless String");

        wifi_driver
            .set_configuration(&Configuration::Client(ClientConfiguration {
                // See .cargo/config.toml to set WIFI_SSID and WIFI_PWD env variables
                ssid: wifi_ssid_32,
                bssid: None,
                auth_method: AuthMethod::WPA2Personal,
                password: wifi_pwd_64,
                channel: None,
                scan_method: ScanMethod::FastScan,
                pmf_cfg: PmfConfiguration::NotCapable,
            }))
            .expect("Failed to set wifi driver configuration");

        wifi_driver.start().expect("Failed to start wifi driver");

        loop {
            match wifi_driver.is_connected() {
                Ok(true) => {
                    #[cfg(debug_assertions)]
                    println!("Wifi is connected");
                    break;
                }
                Ok(false) => {
                    #[cfg(debug_assertions)]
                    println!("Waiting for Wifi connection")
                }
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    println!("Failed to connect wifi driver: {_e:?}")
                }
            }
            if let Err(_e) = wifi_driver.connect() {
                #[cfg(debug_assertions)]
                println!("Error while connecting wifi driver: {_e:?}")
            }

            FreeRtos::delay_ms(1000);
        }

        wifi_driver
    }
}