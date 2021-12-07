use std::io::{ Read, Write };
use std::net::TcpStream;

use std::sync::atomic::{ AtomicUsize, AtomicBool, Ordering };

static WORK_RESULT: AtomicUsize = AtomicUsize::new(0);
static WORK_DONE: AtomicBool = AtomicBool::new(false);

fn work_thread(mut data: Vec<u8>) {
    WORK_DONE.store(false, Ordering::SeqCst);

    for i in 0..256 {
        println!("Day {}", i + 1);
        let mut new_fishes = Vec::new();

        for fish in data.iter_mut() {
            *fish -= 1;
            if *fish <= 0 {
                *fish = 7;
                new_fishes.push(8);
            }
        }

        data.extend(new_fishes);
    }

    WORK_RESULT.store(data.len(), Ordering::SeqCst);
    WORK_DONE.store(true, Ordering::SeqCst);
}

pub fn start(name: String) {
    let mut stream = TcpStream::connect("192.168.1.150:1234")
        .expect("Failed to connect to server");

    stream.set_nonblocking(true)
        .expect("Failed to set non-blocking");

    let mut working = false;
    loop {
        let mut buffer = [0u8; 1024];
        let res = stream.read(&mut buffer);

        if let Ok(len) = res {
            if len == 0 {
                println!("Server exited?");
                return;
            }
            let cmd = buffer[0];

            // TODO(patrik): When executing a command we need to
            // check the if there is remaining bytes inside the buffer
            // because their could be another cmd in there

            match cmd {
                0x01 => {
                    let mut bytes = Vec::new();
                    bytes.push(name.len() as u8);
                    bytes.extend(name.as_bytes());
                    stream.write(&bytes);
                },

                0x02 => {
                    let work_done = WORK_DONE.load(Ordering::SeqCst);

                    let mut status = 0;
                    if working {
                        if work_done {
                            status = 2;
                        } else {
                            status = 1;
                        }
                    }

                    let mut bytes = [0u8];
                    bytes[0] = status;

                    stream.write(&bytes);
                }

                0x03 => {
                    let data_length = buffer[1] as usize;
                    println!("Got data: {}", data_length);

                    let mut data: Vec<u8> =
                        Vec::with_capacity(data_length);
                    data.extend(&buffer[2..2+data_length]);
                    println!("Data: {:?}", data);

                    println!("Spawning work thread");
                    std::thread::spawn(move || {
                        work_thread(data);
                    });

                    working = true;

                    stream.write(&[1]);
                }

                0x04 => {
                    let has_result = WORK_DONE.load(Ordering::SeqCst);

                    if has_result {
                        let result = WORK_RESULT.load(Ordering::SeqCst);
                        let result = result as u64;

                        let mut bytes = Vec::new();
                        bytes.push(1);
                        bytes.extend(result.to_le_bytes());
                        stream.write(&bytes);
                    } else {
                        let bytes = [0u8];
                        stream.write(&bytes);
                    }
                }

                _ => panic!("Unknown cmd: {:#x}", cmd),
            }
        }
    }
}
