
mod server;
mod client;

fn main() {
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() > 1 && args[1] == "client" {
        let name = if args.len() == 3 {
            String::from(args[2].clone())
        } else {
            String::from("Unknown Name")
        };
        println!("Starting client");
        client::start(name);
    } else {
        println!("Starting server");
        server::start();
    }
}

