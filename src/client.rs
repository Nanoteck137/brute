use std::io::{ Read, Write };
use std::net::TcpStream;

use std::sync::atomic::{ AtomicUsize, AtomicBool, Ordering };

static WORK_RESULT: AtomicUsize = AtomicUsize::new(0);
static WORK_DONE: AtomicBool = AtomicBool::new(false);

fn do_work(start_day: usize, thread_id: usize,
           num_iter: usize, data: &mut Vec<i8>) {
    for i in 0..num_iter {
        // println!("{}# Day {}", thread_id, start_day + i + 1);

        let mut new_fishes = Vec::new();

        for fish in data.iter_mut() {
            *fish -= 1;
            if *fish < 0 {
                *fish = 6;
                new_fishes.push(8);
            }
        }

        data.extend(new_fishes);
    }
}

fn work_thread(num_threads: usize, mut data: Vec<i8>) {
    const NUM_ITER: usize = 256;
    const SPLIT_POINT: usize = 150;

    WORK_DONE.store(false, Ordering::SeqCst);
    WORK_RESULT.store(0, Ordering::SeqCst);

    for i in 0..SPLIT_POINT {
        do_work(i + 1, 0, 1, &mut data);
    }

    // Split the work up for more threads
    let data_length = data.len();
    let chunk_size = data_length / num_threads;
    let remaining = data_length % num_threads;

    println!("Spliting up work: Chunk size - {}", chunk_size);

    let mut offset = 0;
    let mut thread_id = 0;

    let mut threads = Vec::new();

    for i in 0..num_threads {
        let length = if i == 0 {
            chunk_size + remaining
        } else {
            chunk_size
        };

        let chunk = &data[offset..offset+length];

        let mut thread_data: Vec<i8> = Vec::new();
        thread_data.extend(chunk);

        let handle = std::thread::spawn(move || {
            let mut data = thread_data;
            do_work(SPLIT_POINT, thread_id,
                    NUM_ITER - SPLIT_POINT, &mut data);

            WORK_RESULT.fetch_add(data.len(), Ordering::SeqCst);
        });

        threads.push(handle);

        thread_id += 1;
        offset += length;
    }

    for handle in threads {
        handle.join()
            .expect("Thread panic");
    }

    println!("Threads done?");

    WORK_DONE.store(true, Ordering::SeqCst);
}

pub fn start(server_address: String, name: String, num_threads: usize) {
    let mut stream = TcpStream::connect(server_address)
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

                    let mut thread_data = Vec::with_capacity(data_length);
                    for num in data {
                        thread_data.push(num as i8);
                    }

                    println!("Spawning work thread");
                    std::thread::spawn(move || {
                        work_thread(num_threads, thread_data);
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

                        working = false;
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
