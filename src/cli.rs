use crate::{config, CONFIG};
use io::BufRead;
use std::io;

// TODO: re add commands to modify config through cli (also overhaul the entire thing, maybe add colors)
pub fn start() {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let mut parts = line.split_whitespace();

        let command = parts.next().unwrap().to_lowercase();
        match command.as_str() {
            // "list" => execute_list(&command, &mut parts),
            // "forward" => execute_forward(&command, &mut parts),
            "reload" => execute_reload(&command, &mut parts),

            _ => println!("Unknown command '{}'", command),
        }
    }
}

// fn execute_list<'i, A: Iterator<Item = &'i str>>(_command: &str, _args: &'i mut A) {
//     let config = CONFIG.read().unwrap();

//     println!("virtual hosts:");
//     for vhost in config.virtualhosts.iter() {
//         print!("  {} ", vhost.hostname);
//         match &vhost.target {
//             HostTarget::Forward(target) => println!("> {}", target),
//             HostTarget::Status {
//                 online_players,
//                 max_players,
//                 description,
//             } => println!("# {:?} - {}/{}", description, online_players, max_players),
//         }
//     }
// }

// fn execute_forward<'i, A: Iterator<Item = &'i str>>(_command: &str, args: &'i mut A) {
//     let hostname = args.next().map(|s| s.parse());
//     let target = args.next().map(|s| s.parse());

//     if let (Some(hostname), Some(target)) = (hostname, target) {
//         if hostname.is_err() {
//             println!("hostname is invalid");
//             return;
//         }

//         if target.is_err() {
//             println!("target is invalid");
//             return;
//         }

//         {
//             let mut config = CONFIG.write().unwrap();
//             (*config).forwards.push(Forward {
//                 hostname: hostname.unwrap(),
//                 target: target.unwrap(),
//             });
//         }

//         config::save(&CONFIG.read().unwrap()).unwrap();
//         println!("new forward created");
//     } else {
//         println!("usage: forward <hostname> <target>");
//     }
// }

fn execute_reload<'i, A: Iterator<Item = &'i str>>(_command: &str, _args: &'i mut A) {
    *CONFIG.write().unwrap() = config::load().unwrap();
    println!("> Reloaded config");
}
