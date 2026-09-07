use std::{
    ffi::{c_uint, c_void},
    fs::File,
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd, IntoRawFd},
        unix::net::{UnixListener, UnixStream},
    },
    process::Command,
    time::Duration,
};

fn main() {
    let subcommand = std::env::args().nth(1);
    match subcommand.as_deref() {
        Some("send") => send(),
        Some("recv") => recv(),
        None => {
            eprintln!("Either 'send' or 'recv' as the first argument");
            std::process::exit(-1);
        }
        Some(cmd) => {
            eprintln!("Action '{cmd}' not recognized. Either 'send' or 'recv'");
            std::process::exit(-1);
        }
    }
}

const SOCKET_PATH: &'static str = "socket";
const MSGDATA: &'static str = "msgdata";
const MSGDATA_LEN: usize = MSGDATA.len();

fn send() -> ! {
    // Remove the socket file if it exists.
    std::fs::remove_file(SOCKET_PATH).ok();

    // Open a Unix Socket (AF_UNIX) where we will send the fd
    let us = UnixListener::bind(SOCKET_PATH).unwrap();
    println!("[SEND] Opened UnixSocket for recv");

    // Launch the child binary
    let mut cmd = Command::new("target/debug/january");
    cmd.arg("recv");
    let mut child = cmd.spawn().unwrap();
    println!("[SEND] Launched child binary");

    // Open our testfile and read the first three bytes. This is the fd we will send
    let mut file = File::open("testfile").unwrap();
    let mut read_msg = String::from("[SEND] Read from file: ");
    read_three_from_file(&mut file, &mut read_msg);
    println!("{read_msg}");

    // Grab the raw file descriptor (and store the length of the type for later)
    let file_fd = file.into_raw_fd();
    let file_fd_len = std::mem::size_of_val(&file_fd) as c_uint;

    let mut io = libc::iovec {
        iov_base: MSGDATA.as_ptr() as *mut c_void,
        iov_len: MSGDATA_LEN,
    };

    let cmsg_len = unsafe { libc::CMSG_SPACE(file_fd_len) };
    let mut buffer = vec![0; cmsg_len as usize];

    let msg = libc::msghdr {
        msg_iov: &mut io,
        msg_iovlen: 1,
        msg_control: buffer.as_mut_ptr() as *mut c_void,
        msg_controllen: buffer.len() as u32,

        // Not in the example gist? What are these?
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_flags: 0,
    };

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    unsafe {
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(file_fd_len);
        let cmsg_data = libc::CMSG_DATA(cmsg) as *mut i32;

        let cmsg_slice = std::slice::from_raw_parts_mut(cmsg_data, 1);
        cmsg_slice[0] = file_fd;
    }

    let (client, _) = us.accept().unwrap();
    let client_fd = client.as_raw_fd();

    println!("[SEND] Attempting to sendmsg");
    unsafe {
        let sent = libc::sendmsg(client_fd, &msg, 0);
        if sent as usize != MSGDATA_LEN {
            eprintln!("Expected to send {MSGDATA_LEN} bytes, actually sent {sent}");
            child.kill().unwrap();
            std::process::exit(-1);
        }
    }

    println!("[SEND] Sleeping for 2 seconds for recv to get message");
    std::thread::sleep(Duration::from_secs(2));

    std::process::exit(0);
}

fn recv() -> ! {
    let us = UnixStream::connect(SOCKET_PATH).unwrap();
    let us_fd = us.as_raw_fd();
    let us_fd_len = std::mem::size_of_val(&us_fd) as c_uint;
    println!("[RECV] Opened UnixStream to send");

    let mut io_buffer = [0u8 as libc::c_char; MSGDATA_LEN];
    let mut io = libc::iovec {
        iov_base: io_buffer.as_mut_ptr() as *mut c_void,
        iov_len: MSGDATA_LEN,
    };

    // Using `us_fd` here just as a stand in for the type of a file descriptor
    let cmsg_len = unsafe { libc::CMSG_SPACE(us_fd_len) };
    let mut cmsg_buffer = vec![0; cmsg_len as usize];
    let mut msg = libc::msghdr {
        msg_iov: &mut io,
        msg_iovlen: 1,
        msg_control: cmsg_buffer.as_mut_ptr() as *mut c_void,
        msg_controllen: cmsg_buffer.len() as u32,

        // Not in the example gist? What are these?
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_flags: 0,
    };

    println!("[RECV] Attempting to recvmsg");
    unsafe {
        let recv = libc::recvmsg(us_fd, &mut msg, libc::MSG_WAITALL);
        if recv as usize != MSGDATA_LEN {
            eprintln!("Expected to recv {MSGDATA_LEN} bytes, got {recv} bytes");
            std::process::exit(-1);
        }
    }

    print!("[RECV] Message received over Unix Socket: ");
    for by in io_buffer {
        print!("{}", by as u8 as char);
    }
    println!();

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&mut msg) };
    let cmsg_data = unsafe { libc::CMSG_DATA(cmsg) as *mut i32 };
    let cmsg_slice = unsafe { std::slice::from_raw_parts(cmsg_data, 1) };
    let recv_fd = cmsg_slice[0];
    println!("[RECV] Received file descriptor: {recv_fd}");

    let mut file = unsafe { File::from_raw_fd(recv_fd) };
    let mut read_msg = String::from("[RECV] Read from file: ");
    read_three_from_file(&mut file, &mut read_msg);
    println!("{read_msg}");

    println!("recv proccess exiting!");
    std::process::exit(0);
}

fn read_three_from_file(file: &mut File, buffer: &mut String) {
    let mut buf = [0u8; 3];
    file.read_exact(&mut buf).unwrap();

    for by in buf {
        buffer.push(by as char);
    }
}
