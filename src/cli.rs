use std::io;
use io::BufRead;
use crate::{config::{self, Forward}, CONFIG};

pub fn start() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let mut parts = line.split_whitespace();

        let command = parts.next().unwrap().to_lowercase();
        match command.as_str() {
            "list" => {
                let config = CONFIG.read().unwrap();

                println!("forwards:");
                for forward in config.forwards.iter() {
                    println!("{} -> {}", forward.hostname, forward.target);
                }
            }

            "forward" => {
                let hostname = parts.next();
                let target = parts.next();

                if let (Some(hostname), Some(target)) = (hostname, target) {
                    {
                        let mut config = CONFIG.write().unwrap();
                        (*config).forwards.push(Forward {
                            hostname: hostname.to_string(),
                            target: target.to_string(),
                        });
                    }

                    config::save(&CONFIG.read().unwrap()).unwrap();
                } else {
                    println!("usage: forward <hostname> <target>");
                }
            }

            "reload" => {
                *CONFIG.write().unwrap() = config::load().unwrap();
                println!("reloaded forwards");
            }

            _ => println!("Unknown command '{}'", command),
        }
    }
}