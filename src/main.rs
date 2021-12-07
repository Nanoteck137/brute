mod server;
mod client;

use clap::{AppSettings, Parser};

#[derive(Parser)]
#[clap(version = "1.0", author = "Patrik M. Rosenström <patrik.millvik@gmail.com>")]
struct Opts {
    #[clap(subcommand)]
    command: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    Client(ClientOpts),
    Server(ServerOpts),
}

#[derive(Parser)]
struct ClientOpts {
    /// The IP Address to the server (Example: 192.168.1.1:1234)
    server_address: String,

    /// Name of the client (optional)
    name: Option<String>,

    /// Number of threads to use (optional)
    #[clap(short)]
    threads: Option<usize>,
}

#[derive(Parser)]
struct ServerOpts {
    /// The binding address the server should use
    bind_address: String,
}

fn main() {
    let opts = Opts::parse();

    match opts.command {
        SubCommand::Client(client_opts) => {
            let name = if let Some(name) = client_opts.name {
                name
            } else {
                let hostname = sys_info::hostname()
                    .expect("Failed to find hostname");
                let os = sys_info::os_type()
                    .expect("Failed to find os type");
                format!("{} - {}", os, hostname)
            };

            let server_addr = client_opts.server_address;

            let num_threads = if let Some(num_threads) = client_opts.threads {
                num_threads
            } else {
                sys_info::cpu_num()
                    .expect("Failed to find the number of cpus of the system")
                        as usize
            };

            println!("Starting client: {} - Num Threads: {}", name, num_threads);
            client::start(server_addr, name, num_threads);
        },

        SubCommand::Server(server_opts) => {
            let bind_addr = server_opts.bind_address;
            println!("Starting server: {}", bind_addr);
            server::start(bind_addr);
        }
    }

    /*
    let args = std::env::args().collect::<Vec<String>>();

    if args.len() > 1 && args[1] == "client" {
        let name = if args.len() == 3 {
            String::from(args[2].clone())
        } else {
            let hostname = sys_info::hostname()
                .expect("Failed to find hostname");
            let os = sys_info::os_type()
                .expect("Failed to find os type");
            format!("{} - {}", os, hostname)
        };
        println!("Starting client: {}", name);
        client::start(name);
    } else {
        println!("Starting server");
        server::start("192.168.1.150:1234".to_string());
    }
    */
}

