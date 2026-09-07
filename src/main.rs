use std::{
    ffi::CString,
    os::{
        fd::AsRawFd,
        unix::net::{UnixListener, UnixStream},
    },
    process::Command,
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
fn send() -> ! {
    let us = UnixListener::bind(SOCKET_PATH).unwrap();

    // Launch the child binary
    let mut cmd = Command::new("target/debug/january");
    cmd.arg("recv");
    let mut child = cmd.spawn().unwrap();

    let mut fds = [0 as libc::c_int; 2];
    let result = unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, fds.as_mut_ptr()) };
    if result == -1 {
        eprintln!("Failed to spawn socketpair");
        child.kill().unwrap();
        std::process::exit(-1);
    }

    unsafe {
        libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        libc::fcntl(fds[1], libc::F_SETFL, libc::O_NONBLOCK);
    }

    let msg_content = CString::new("ab").unwrap();
    let mut io = libc::iovec {
        iov_base: msg_content.as_ptr() as *mut libc::c_void,
        iov_len: msg_content.count_bytes(),
    };
    let cmsg_len = unsafe { libc::CMSG_SPACE(std::mem::size_of_val(&fds[1]) as libc::c_uint) };
    let mut buffer = vec![0; cmsg_len as usize];

    let msg = libc::msghdr {
        msg_iov: &mut io,
        msg_iovlen: 1,
        msg_control: buffer.as_mut_ptr() as *mut libc::c_void,
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
        (*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of_val(&fds[1]) as libc::c_uint);
        let cmsg_data = libc::CMSG_DATA(cmsg) as *mut i32;
        *cmsg_data = fds[1];
    }

    let (client, _) = us.accept().unwrap();
    let client_fd = client.as_raw_fd();

    unsafe {
        let sent = libc::sendmsg(client_fd, &msg, 0);
        if sent != 2 {
            eprintln!("Expected to send 2 bytes, actually sent {sent}");
            child.kill().unwrap();
            std::process::exit(-1);
        }

        let write_content = CString::new("yz").unwrap();
        libc::write(fds[0], write_content.as_ptr() as *mut libc::c_void, 2);
    }

    std::process::exit(0);
}

fn recv() -> ! {
    let us = UnixStream::connect(SOCKET_PATH).unwrap();
    let us_fd = us.as_raw_fd();

    let mut io_buffer = [0u8 as libc::c_char; 2];
    let mut io = libc::iovec {
        iov_base: io_buffer.as_mut_ptr() as *mut libc::c_void,
        iov_len: 2,
    };

    // Using `us_fd` here just as a stand in for the type of a file descriptor
    let cmsg_len = unsafe { libc::CMSG_SPACE(std::mem::size_of_val(&us_fd) as libc::c_uint) };
    let mut cmsg_buffer = vec![0; cmsg_len as usize];
    let mut msg = libc::msghdr {
        msg_iov: &mut io,
        msg_iovlen: 1,
        msg_control: cmsg_buffer.as_mut_ptr() as *mut libc::c_void,
        msg_controllen: cmsg_buffer.len() as u32,

        // Not in the example gist? What are these?
        msg_name: std::ptr::null_mut(),
        msg_namelen: 0,
        msg_flags: 0,
    };

    unsafe {
        let recv = libc::recvmsg(us_fd, &mut msg, libc::MSG_WAITALL);
        if recv != 2 {
            eprintln!("Expected to recv 2 bytes, got {recv} bytes");
            std::process::exit(-1);
        }
    }

    print!("Message received over Unix Socket: ");
    for by in io_buffer {
        print!("{}", by as u8 as char);
    }
    println!();

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&mut msg) };
    let cmsg_data = unsafe { libc::CMSG_DATA(cmsg) as *mut i32 };
    let recv_fd = unsafe { *cmsg_data };
    println!("Received file descriptor: {recv_fd}");

    let mut recv_buffer = [0u8 as libc::c_char; 2];
    unsafe {
        let recv = libc::read(recv_fd, recv_buffer.as_mut_ptr() as *mut libc::c_void, 2);
        if recv != 2 {
            eprintln!("[SENTFD] Expected to recv 2 bytes, got {recv} bytes");
            std::process::exit(-1);
        }
    }

    print!("Message received over Sent Descriptor: ");
    for by in recv_buffer {
        print!("{}", by as u8 as char);
    }
    println!();

    println!("recv proccess exiting!");
    std::process::exit(0);
}
