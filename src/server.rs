use std::io::{ Read, Write };
use std::net::{ TcpListener, TcpStream };
use std::sync::{ Mutex, Arc };
use std::sync::atomic::{ AtomicUsize, Ordering };
use std::collections::VecDeque;

// Commands:
//   - 0x01: Identify
//   - 0x02: Status (0: Waiting for work
//                   1: Running work)
//   - 0x03: Send data
//   - 0x04: Request result
//
// Client connects to the server
// The server handles all the work queues
// The server sends a work request to the client
//

const CMD_IDENTITY:       u8 = 0x01;
const CMD_STATUS:         u8 = 0x02;
const CMD_SEND_DATA:      u8 = 0x03;
const CMD_REQUEST_RESULT: u8 = 0x04;

#[derive(Copy, Clone, PartialEq, Debug)]
enum Status {
    Waiting,
    Running,
    Done,
}

static CLIENT_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct Work {
    data: Vec<u8>,
    result: Option<u64>,
}

struct Client {
    id: usize,
    stream: TcpStream,
    name: String,
}

impl Client {
    fn init(&mut self) {
        self.stream.set_nonblocking(true)
            .expect("Failed to set non-blocking on client");
    }

    fn send_work(&mut self, work: &Work) -> Option<()> {
        // let cmd = [0x02u8];
        let mut cmd = Vec::new();
        cmd.push(CMD_SEND_DATA);
        cmd.push(work.data.len() as u8);
        cmd.extend(&work.data[..]);
        self.stream.write(&cmd);

        let mut status = [0u8; 1];
        let length = loop {
            let res = self.stream.read(&mut status);
            if let Ok(len) = res {
                break len;
            }
        };

        assert!(length == 1 || length == 0);

        if length == 0 {
            return None;
        }

        if status[0] == 1 {
            println!("Client accepted work");
        } else {
            println!("Client didn't accept the work");
        }

        return Some(());
    }

    fn block_read(&mut self, buffer: &mut [u8]) -> Option<usize> {
        loop {
            let res = self.stream.read(buffer);
            if let Ok(len) = res {
                if len == 0 {
                    return None;
                }

                return Some(len);
            }
        }
    }

    fn get_result(&mut self) -> Option<u64> {
        let cmd = [CMD_REQUEST_RESULT];
        self.stream.write(&cmd);

        let mut status = [0u8];
        let len = self.block_read(&mut status)?;
        assert!(len == 1);

        if status[0] == 1 {
            let mut data = [0u8; 8];
            let len = self.block_read(&mut data)?;
            println!("Len: {}", len);
            assert!(len == 8);

            let result = u64::from_le_bytes(data);
            return Some(result);
        } else {
            return None;
        }

        return None;
    }

    fn get_status(&mut self) -> Option<Status> {
        let cmd = [0x02u8];
        self.stream.write(&cmd);

        let mut status = [0u8; 1];
        let len = self.block_read(&mut status)?;
        assert!(len == 1);

        match status[0] {
            0 => return Some(Status::Waiting),
            1 => return Some(Status::Running),
            2 => return Some(Status::Done),

            _ => panic!("Unknown status"),
        }

        return None;
    }

    fn identify(&mut self) -> Option<()> {
        let cmd = [0x01u8];
        self.stream.write(&cmd);

        let mut name_len = [0u8];
        let len = self.block_read(&mut name_len)?;
        assert!(len == 1);

        let mut name: Vec<u8> = vec![0u8; name_len[0] as usize];
        let len = self.block_read(&mut name[..])?;
        assert!(len == name.len());

        let name = std::str::from_utf8(&name).unwrap();
        self.name = String::from(name);

        Some(())
    }
}

fn new_client_id() -> usize {
    CLIENT_ID.fetch_add(1, Ordering::SeqCst)
}

/*
fn handle_clients(clients: Arc<Mutex<Vec<Client>>>) {
    loop {
        {
            let mut lock = clients.lock().unwrap();
            println!("Num clients: {}", lock.len());

            for client in lock.iter_mut() {
                handle_connection(client);
            }
        }

        std::thread::sleep_ms(2000);
    }
}
*/

fn load_data() -> Vec<u8> {
    let data = std::fs::read_to_string("data.txt")
        .expect("Failed to load data.txt");
    let data = data.trim();

    let mut result = Vec::new();
    for split in data.split(",") {
        let num = split.parse::<u8>()
            .expect("Failed to parse split string");

        result.push(num);
    }

    result
}

fn prepare_work_queue(data: Vec<u8>) -> (Arc<Mutex<VecDeque<Work>>>, usize) {
    let mut result = VecDeque::new();

    for num in data {
        result.push_back(Work {
            data: vec![num],
            result: None,
        });
    }

    let num_entries = result.len();

    (Arc::new(Mutex::new(result)), num_entries)
}

pub fn start() {
    let data = load_data();
    let work_done = Arc::new(Mutex::new(Vec::<Work>::new()));
    let (work_queue, num_queue_entries) = prepare_work_queue(data);

    let listener = TcpListener::bind("192.168.1.150:1234")
        .expect("Failed to bind TcpListener to '127.0.0.1:1234'");

    /*
    let clients = Arc::new(Mutex::new(Vec::new()));

    let clients_clone = clients.clone();
    std::thread::spawn(move || {
        handle_clients(clients_clone);
    });
    */

    let queue = work_queue.clone();
    let done_queue = work_done.clone();
    std::thread::spawn(move || {
        loop {
            {
                let lock = queue.lock().unwrap();
                if lock.len() <= 0 {
                    println!("Work queue empty");
                } else {
                    println!("There is still {} work requests", lock.len());
                }
            }

            {
                let lock = done_queue.lock().unwrap();
                if lock.len() >= num_queue_entries {
                    println!("Processed all of the work queue");

                    let mut sum = 0u64;
                    for result in lock.iter() {
                        sum += result.result.unwrap();
                    }

                    println!("Answer: {}", sum);
                }
            }

            std::thread::sleep_ms(2000);
        }
    });

    for stream in listener.incoming() {
        println!("Connection");

        let stream = stream.unwrap();

        let id = new_client_id();
        let mut client = Client {
            id,
            stream,
            name: "".to_string(),
        };

        let queue = work_queue.clone();
        let done_queue = work_done.clone();
        std::thread::spawn(move || {
            handle_connection(queue, done_queue, &mut client);
            println!("Disconnecting client: {}", client.name);
        });

        // clients.lock().unwrap().push(client);

        // handle_connection(stream);
    }
}

fn handle_connection(work_queue: Arc<Mutex<VecDeque<Work>>>,
                     done_queue: Arc<Mutex<Vec<Work>>>,
                     client: &mut Client)
    -> Option<()>
{
    client.init();
    client.identify();

    println!("Client Name: {}", client.name);

    let mut current_work = None;

    loop {
        let status = client.get_status()?;
        match status {
            Status::Waiting => {
                println!("{}: Need to send client some work", client.name);
                let work = {
                    work_queue.lock().unwrap().pop_front()?
                };
                client.send_work(&work);

                current_work = Some(work);
            }

            Status::Running => {
                println!("{}: Client is doing work", client.name);
            }

            Status::Done => {
                println!("{}: Need to retrive work result", client.name);

                let result = client.get_result()?;
                println!("Got result: {}", result);

                let mut work = current_work.take()
                    .expect("No current work?");
                work.result = Some(result);
                {
                    done_queue.lock().unwrap().push(work);
                }
            }
        }

        std::thread::sleep_ms(5000);
    }
}
