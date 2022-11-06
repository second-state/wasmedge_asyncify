use std::thread::sleep;
use std::time::Duration;
fn main() {
    env_logger::init();
    let mut delay = Duration::from_secs(10);
    for i in 0..=5 {
        if call_some_api(i) {
            break;
        }
        if i < 3 {
            delay = delay * 2;
        } else {
            delay += Duration::from_secs(60);
        }
        log::info!("sleep {}s", delay.as_secs());
        sleep(delay);
    }
}

fn call_some_api(id: u32) -> bool {
    let mut writer = Vec::new();
    // call some http api
    let r = http_req::request::get(format!("http://httpbin.org/get?id={}", id), &mut writer);
    if let Err(e) = r {
        log::error!("http error:{}", e);
    } else {
        log::info!("body len= {}", writer.len());
    }
    // fake logic
    id == 5
}
