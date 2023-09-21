pub mod analytic;

use crate::config::Config;
use reqwest::{RequestBuilder, StatusCode};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::{task::yield_now, time};

pub struct ShootRes {
    id: u64,
    timestamp: u128,
    iter_id: u64,
    result: Option<String>,
    err: bool,
    latency: Option<u128>,
    http_status: Option<StatusCode>,
}

impl ShootRes {
    fn new() -> Self {
        ShootRes {
            id: 0,
            timestamp: 0,
            iter_id: 0,
            result: None,
            err: false,
            latency: None,
            http_status: None,
        }
    }

    pub fn get_latency_ms(&self) -> Option<u128> {
        self.latency
    }

    pub fn timestamp(&self) -> u128 {
        self.timestamp
    }

    pub fn is_err(&self) -> bool {
        self.err
    }

    pub fn show_res(&self) -> String {
        match &self.result {
            Some(res_str) => res_str.clone(),
            None => String::from("Not firing yet..."),
        }
    }

    pub fn dedicate_self(self) -> Self {
        self
    }
}

pub async fn fire(gun_id: u64, arc_conf: Arc<Config>) -> Vec<ShootRes> {
    // Init ShootResult object
    let mut shoot_res_list: Vec<ShootRes> = Vec::new();

    let conf = arc_conf.as_ref();
    let met = conf.method.clone();
    let cli = reqwest::Client::new();
    if conf.verbose {
        println!("GUN#{gun_id} Start shooting at {}", conf.url);
    }
    for i in 0..conf.repeat {
        let mut shoot_res = ShootRes::new();
        shoot_res.id = gun_id;
        shoot_res.iter_id = i;

        let mut req_builder: RequestBuilder = cli.request(met.clone(), &conf.url);
        if let Some(header) = conf.bullet.get_header() {
            req_builder = req_builder.headers(header.clone()); // Use RC? since clone will take a lot of time
        }
        if let Some(form) = conf.bullet.get_form() {
            req_builder = req_builder.form(form);
        }
        if let Some(body) = conf.bullet.get_body() {
            req_builder = req_builder.body(body.clone());
        }

        // Timer here
        let begin_fire_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        let resp = req_builder.send().await;
        let end_fire_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        shoot_res.timestamp = begin_fire_time.as_nanos();
        shoot_res.latency = Some(end_fire_time.as_millis() - begin_fire_time.as_millis());

        yield_now().await;

        // Add delay here
        if let Some(delay_ms) = conf.delay {
            if conf.verbose {
                println!("GUN#{gun_id}[{i}]|Delay for {delay_ms} ms.");
            }
            tokio::time::sleep(time::Duration::from_millis(delay_ms)).await;
        }

        match resp {
            Ok(r) => {
                let lat = match shoot_res.get_latency_ms() {
                    Some(lat) => format!("{lat}"),
                    None => format!("N/A"),
                };
                shoot_res.http_status = Some(r.status());
                let r_text = format!(
                    "GUN#{gun_id}[{i}]|{:?}->Got in {lat} ms|{:?}",
                    met,
                    r.text().await
                );
                if conf.verbose {
                    println!("{r_text}")
                }
                shoot_res.result = Some(r_text);
            }
            Err(e) => {
                let e_text = format!("GUN#{gun_id}[{i}]|{:?}->Err{:?}", met, e);
                if conf.verbose {
                    println!("{e_text}")
                }
                shoot_res.result = Some(e_text);
                shoot_res.err = true;
            }
        }
        shoot_res_list.push(shoot_res);
    }
    shoot_res_list
}
