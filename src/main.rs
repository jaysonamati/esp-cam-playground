mod wifi;
mod rgb;
mod espcam;
mod wifi_conf;

use anyhow::{bail, Ok as Oky, Result};
use embedded_svc::{http::{client::Client, Method},io::Read,};
use core::str;
// use core::result::Result::Ok;
use std::net::Ipv4Addr;

use esp_idf_hal::{delay::FreeRtos, gpio::PinDriver, io::Write, peripherals::Peripherals};
use esp_idf_svc::{http::{client::{Configuration, EspHttpConnection}, server::{Configuration as ServerConfig, EspHttpServer}}, ping::EspPing};

use crate::{espcam::Camera, wifi::Wifi};

fn main() -> Result<()>{
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, Esp");

    let peripherals = Peripherals::take().expect("Failed to take peripherals");

    let wifi = Wifi::init(peripherals.modem); // Connectivity goes away when dropped

    FreeRtos::delay_ms(5000); // Wait for the DHCP server to deliver a lease

    let gw_addr: Ipv4Addr = wifi
        .sta_netif()
        .get_ip_info()
        .expect("Failed to get ip info")
        .subnet
        .gateway
        .into();

    FreeRtos::delay_ms(2000);

    // This doesn't work as is (See dev-notes 22March)
    let camera = Camera::new(
        peripherals.pins.gpio32,
        peripherals.pins.gpio0,
        peripherals.pins.gpio5,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        peripherals.pins.gpio21,
        peripherals.pins.gpio36,
        peripherals.pins.gpio39,
        peripherals.pins.gpio34,
        peripherals.pins.gpio35,
        peripherals.pins.gpio25,
        peripherals.pins.gpio23,
        peripherals.pins.gpio22,
        peripherals.pins.gpio26,
        peripherals.pins.gpio27,
        esp_idf_sys::camera::pixformat_t_PIXFORMAT_JPEG,
        esp_idf_sys::camera::framesize_t_FRAMESIZE_UXGA,
    )
    .unwrap();
     

    let mut led_r = PinDriver::output(peripherals.pins.gpio33).unwrap();

    /*
    // Make a get http request to Url
    get("http://neverssl.com/")?;

    // Make a get https request to Url
    gets("https://espressif.com/")?;
     */

    // 1.Create a `EspHttpServer` instance using a default configuration
    let mut server = EspHttpServer::new(&ServerConfig::default())?;
    // http://<sta ip>/ handler
    server.fn_handler("/", Method::Get, |request| {
        let html = index_html();
        let mut response = request.into_ok_response()?;
        response.write_all(html.as_bytes())?;
        Oky(())
    })?;

    
    /*
    server.fn_handler("/camera.jpg", Method::Get, |request| {
        let framebuffer = camera.get_framebuffer();

        if let Some(framebuffer) = framebuffer {
            let data = framebuffer.data();

            let headers = [
                ("Content-Type", "image/jpeg"),
                ("Content-Length", &data.len().to_string()),
            ];
            let mut response = request.into_response(200, Some("Ok"), &headers).unwrap();
            response.write_all(data)?;

        } else {
            let mut response = request.into_ok_response()?;
            response.write_all("no framebuffer".as_bytes())?;
        }
        Oky(())

    })?;
     */
    


    println!("Server awaiting connection");

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        led_r.set_low().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1000));
        led_r.set_high().unwrap();

        let ping = EspPing::default()
            .ping(gw_addr, &Default::default())
            .expect("Failed to ping");

        println!("Ping summary: {:?}", ping);
        log::info!("Ping summary: {:?}", ping);
    }
}

// https://github.com/esp-rs/std-training/blob/main/intro/http-server/examples/http_server.rs

fn templated(content: impl AsRef<str>) -> String {
    format!(
        r#"
<!DOCTYPE html>
<html>
    <head>
        <meta charset="utf-8">
        <title>InfDyn Sense web server</title>
    </head>
    <body>
        {}
    </body>
</html>
"#,
        content.as_ref()
    )
}

fn index_html() -> String {
    templated("Hello from Esp Spy Cam!")
}

/*
// use when connected to DHT Sensor
fn temperature(val: f32) -> String {
    templated(format!("Ambient temperature: {:.2}°C", val))
}
 */


// https://github.com/esp-rs/std-training/blob/main/intro/http-client/src/main.rs
fn get(url: impl AsRef<str>) -> Result<()> {

    // 1. Create a new EspHttpConnection with default Configuration.
    let http_connection = EspHttpConnection::new(&Configuration::default())?;

    // 2. Get a client using the embedded_svc Client::wrap method.
    let mut client = Client::wrap(http_connection);

    // 3. Open a GET request to `url`
    let headers = [("accept", "text/plain")];

    // ANCHOR: request
    let request = client.request(Method::Get, url.as_ref(), &headers)?;
    // ANCHOR_END: request

    // 4. Submit the request and check the status code of the response.
    // Successful http status codes are in the 200..=299 range.
    let response = request.submit()?;
    let status = response.status();

    println!("Response code: {}\n", status);
    match status {
        200..=299 => {
            // 5. If the status is OK, read response data chunk by chunk into a buffer and print it until done.
            //
            // NB. There is no guarantee that chunks will be split at the boundaries of valid UTF-8
            // sequences (in fact it is likely that they are not) so this edge case needs to be handled.
            // However, for the purposes of clarity and brevity(?), the additional case of completely invalid
            // UTF-8 sequences will not be handled here and is left for later.
            let mut buf = [0_u8; 256];
            // Offset into the buffer to indicate that there may still be
            // bytes at the beginning that have not been decoded yet
            let mut offset = 0;
            // Keep track of the total number of bytes read to print later
            let mut total = 0;
            let mut reader = response;
            loop {
                // read into the buffer starting at the offset to not overwrite
                // the incomplete UTF-8 sequence we put there earlier
                if let Ok(size) = Read::read(&mut reader, &mut buf[offset..]) {
                    if size == 0 {
                        // It might be nice to check if we have any left over bytes here (ie. the offset > 0)
                        // as this would mean that the response ended with an invalid UTF-8 sequence, but for the
                        // purposes of this training we are assuming that the full response will be valid UTF-8
                        break;
                    }
                    // update the total number of bytes read
                    total += size;
                    // 6. Try converting the bytes into a Rust (UTF-8) string and print it.
                    // Remember that we read into an offset and recalculate the real length
                    // of the bytes to decode.
                    let size_plus_offset = size + offset;
                    match str::from_utf8(&buf[..size_plus_offset]) {
                        Ok(text) => {
                            // buffer contains fully valid UTF-8 data,
                            // print it and reset the offset to 0.
                            print!("{}", text)
                        },
                        Err(error) => {
                            // The buffer contains incomplete UTF-8 data, we will
                            // print the valid part, copy the invalid sequence to
                            // the beginning of the buffer and set an offset for the
                            // next read.
                            //
                            // NB. There is actually an additional case here that should be
                            // handled in a real implementation. The Utf8Error may also contain
                            // an error_len field indicating that there is actually an invalid UTF-8
                            // sequence in the middle of the buffer. Such an error would not be
                            // recoverable through our offset and copy mechanism. The result will be
                            // that the invalid sequence will be copied to the front of the buffer and
                            // eventually the buffer will be filled until no more bytes can be read when
                            // the offset == buf.len(). At this point the loop will exit without reading
                            // any more of the response.
                            let valid_up_to = error.valid_up_to();
                            unsafe {
                                // It's ok to use unsafe here as the error code already told us that
                                // the UTF-8 data up to this point is valid, so we can tell the compiler
                                // it's fine.
                                print!("{}", str::from_utf8_unchecked(&buf[..valid_up_to]));
                            }
                            buf.copy_within(valid_up_to.., 0);
                            offset = size_plus_offset - valid_up_to;
                            
                            // TODO: Handle 3xx, 4xx and 5xx status codes each in a separate match arm
                            // TODO: Write a custom Error enum to represent these errors. Implement the std::error::Error trait for your error.
                        },
                    }
                }
            }
            println!("Total: {} bytes", total);
        }
        _ => {
            bail!("Unexpected response code: {}", status)
        }
    }
    Ok(())
}

fn gets(url: impl AsRef<str>) -> Result<()> {
    // 1. Create a new EspHttpClient.
    // ANCHOR: connection
    let connection = EspHttpConnection::new(&Configuration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    })?;
    // ANCHOR_END: connection
    let mut client = Client::wrap(connection);

    // 2. Open a GET request to `url`
    let headers = [("accept", "text/plain")];
    let request = client.request(Method::Get, url.as_ref(), &headers)?;

    // 3. Submit write request and check the status code of the response.
    // Successful http status codes are in the 200..=299 range.
    let response = request.submit()?;
    let status = response.status();

    println!("Response code: {}\n", status);

    match status {
        200..=299 => {
            // 4. if the status is OK, read response data chunk by chunk into a buffer and print it until done
            //
            // NB. see http_client.rs for an explanation of the offset mechanism for handling chunks that are
            // split in the middle of valid UTF-8 sequences. This case is encountered a lot with the given
            // example URL.
            let mut buf = [0_u8; 256];
            let mut offset = 0;
            let mut total = 0;
            let mut reader = response;
            loop {
                if let Ok(size) = Read::read(&mut reader, &mut buf[offset..]) {
                    if size == 0 {
                        break;
                    }
                    total += size;
                    // 5. try converting the bytes into a Rust (UTF-8) string and print it
                    let size_plus_offset = size + offset;
                    match str::from_utf8(&buf[..size_plus_offset]) {
                        Ok(text) => {
                            print!("{}", text);
                            offset = 0;
                        }
                        Err(error) => {
                            let valid_up_to = error.valid_up_to();
                            unsafe {
                                print!("{}", str::from_utf8_unchecked(&buf[..valid_up_to]));
                            }
                            buf.copy_within(valid_up_to.., 0);
                            offset = size_plus_offset - valid_up_to;
                        }
                    }
                }
            }
            println!("Total: {} bytes", total);
        }
        _ => bail!("Unexpected response code: {}", status),
    }

    Ok(())
}