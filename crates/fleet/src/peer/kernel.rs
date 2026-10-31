//! Asking the kernel which connections a process holds.
//!
//! **`libc` carries the two calls and not `socket_fdinfo`**, so that record's
//! shape is transcribed from `sys/proc_info.h` below. Each piece is `#[repr(C)]`
//! with a `const` assertion on its size, so a field transcribed wrongly is a
//! build failure rather than an offset that reads four bytes of somebody else's
//! port. Spike 12 calibrated these offsets at runtime because it was Python; a
//! layout the compiler agrees with is the better guarantee.
//!
//! Only as far as the two ports. `soi_proto` is a union of seven records and
//! the two that are read — `SOCKINFO_IN` and `SOCKINFO_TCP` — both open with an
//! `in_sockinfo`, so the rest is not transcribed: a wrong size in a part nobody
//! reads is a wrong size all the same.
//!
//! Every call here answers `false` on any error, the only direction
//! `crate::peer` may fail in. Each `unsafe` block carries its own note; the
//! header does not spell the token, because the gate's check for one is a
//! file-wide substring and a file that lost every note would still pass.
//!
//! Darwin-only, at the call site, and **there is no `cfg(target_os)` here on
//! purpose**: a guard would compile the lookup out and leave `PeerOf` answering
//! `false`, which is attribution failing absent on a platform where it should
//! fail to build. Whether a layer should own that is
//! `[platform-differences-layer]` in `docs/contracts/adapters.md`, a person's
//! question this is the third instance of.

use std::mem::size_of;

use libc::{c_int, c_void, proc_fdinfo, proc_pidfdinfo, proc_pidinfo};

use super::PeerOf;

/// `PROC_PIDLISTFDS`, from `sys/proc_info.h` by way of `libc`.
const LIST_FDS: c_int = libc::PROC_PIDLISTFDS;
/// `PROX_FDTYPE_SOCKET`.
const FD_SOCKET: u32 = libc::PROX_FDTYPE_SOCKET as u32;
/// `PROC_PIDFDSOCKETINFO`. Not in `libc`; the header's value.
const FD_SOCKET_INFO: c_int = 3;
/// `SOCKINFO_IN` and `SOCKINFO_TCP` — the two kinds whose protocol record
/// begins with an `in_sockinfo`, which is where the two ports are.
const SOCKINFO_IN: i32 = 1;
const SOCKINFO_TCP: i32 = 2;

/// Bigger than `sizeof(struct socket_fdinfo)` on every darwin this runs on. The
/// call refuses a buffer that is too small and ignores one that is too large,
/// so the only failure this size can have is the one direction that errors.
const BUFFER: usize = 2048;

/// How many open files one process may be asked about in one pass. A process
/// with more is walked in one allocation regardless — the list call answers how
/// many bytes it wanted, and this is only the first guess.
const FDS: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: i64,
    fi_type: i32,
    fi_guardflags: u32,
}
const _: () = assert!(size_of::<ProcFileInfo>() == 24);

/// `struct vinfo_stat` — a copy of `stat64` with static sized fields. Nothing
/// here reads a field of it; it is transcribed because `socket_info` opens with
/// one and everything after it is at its far side.
#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct VinfoStat {
    vst_dev: u32,
    vst_mode: u16,
    vst_nlink: u16,
    vst_ino: u64,
    vst_uid: u32,
    vst_gid: u32,
    vst_atime: i64,
    vst_atimensec: i64,
    vst_mtime: i64,
    vst_mtimensec: i64,
    vst_ctime: i64,
    vst_ctimensec: i64,
    vst_birthtime: i64,
    vst_birthtimensec: i64,
    vst_size: i64,
    vst_blocks: i64,
    vst_blksize: i32,
    vst_flags: u32,
    vst_gen: u32,
    vst_rdev: u32,
    vst_qspare: [i64; 2],
}
const _: () = assert!(size_of::<VinfoStat>() == 136);

#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct SockbufInfo {
    sbi_cc: u32,
    sbi_hiwat: u32,
    sbi_mbcnt: u32,
    sbi_mbmax: u32,
    sbi_lowat: u32,
    sbi_flags: i16,
    sbi_timeo: i16,
}
const _: () = assert!(size_of::<SockbufInfo>() == 24);

/// `struct socket_fdinfo`, as far as the two ports and no further.
///
/// The union that follows `rfu_1` in the header is `soi_proto`; its first two
/// members both begin with `struct in_sockinfo`, whose first two fields are the
/// foreign port and the local port. **In that order** — the header puts
/// `insi_fport` first, and reading them the other way round is exactly the
/// mistake this whole module exists to not make.
#[repr(C)]
#[derive(Clone, Copy)]
// Every field is transcribed and only three are read. The rest are here to
// place the three, and removing one would move them.
#[allow(dead_code)]
struct SocketPorts {
    pfi: ProcFileInfo,
    soi_stat: VinfoStat,
    soi_so: u64,
    soi_pcb: u64,
    soi_type: i32,
    soi_protocol: i32,
    soi_family: i32,
    soi_options: i16,
    soi_linger: i16,
    soi_state: i16,
    soi_qlen: i16,
    soi_incqlen: i16,
    soi_qlimit: i16,
    soi_timeo: i16,
    soi_error: u16,
    soi_oobmark: u32,
    soi_rcv: SockbufInfo,
    soi_snd: SockbufInfo,
    soi_kind: i32,
    rfu_1: u32,
    /// `in_sockinfo::insi_fport` — the port at the far end, which for a Drone's
    /// connection to Fleet is Fleet's own listening port.
    insi_fport: i32,
    /// `in_sockinfo::insi_lport` — the port this process opened from, which is
    /// what the listener saw as its peer.
    insi_lport: i32,
}
const _: () = assert!(size_of::<SocketPorts>() == 272);
const _: () = assert!(size_of::<SocketPorts>() <= BUFFER);

/// The real answer: `proc_pidfdinfo` over the sockets one process holds.
#[derive(Debug, Default)]
pub struct Kernel;

impl PeerOf for Kernel {
    fn holds(&self, pid: u32, from: u16, to: u16) -> bool {
        sockets_of(pid)
            .into_iter()
            .filter_map(|fd| ports_of(pid, fd))
            .any(|(lport, fport)| lport == from && fport == to)
    }

    fn children(&self, pid: u32) -> Vec<u32> {
        children_of(pid)
    }

    fn executable(&self, pid: u32) -> Option<String> {
        executable_of(pid)
    }

    fn carries(&self, pid: u32, key: &str, value: &str) -> bool {
        let entry = format!("{key}={value}");
        environment_of(pid).iter().any(|one| *one == entry)
    }
}

/// `PROC_PPID_ONLY`, from `sys/proc_info.h`. Not in `libc`.
const PPID_ONLY: u32 = 6;

/// How many children one process may be asked about in one pass.
const CHILDREN: usize = 64;

/// Every process whose parent is `pid`. Empty on any failure.
fn children_of(pid: u32) -> Vec<u32> {
    let mut pids: Vec<libc::pid_t> = vec![0; CHILDREN];
    // SAFETY: the pointer is to `pids`, live for this call, and the size handed
    // over is that allocation's own. The kernel writes no more than it is told
    // and answers how many bytes it wrote; a pid with no children writes none.
    #[allow(unsafe_code)]
    let bytes = unsafe {
        libc::proc_listpids(
            PPID_ONLY,
            pid,
            pids.as_mut_ptr().cast::<c_void>(),
            (size_of::<libc::pid_t>() * CHILDREN) as c_int,
        )
    };
    if bytes <= 0 {
        return Vec::new();
    }
    let held = (bytes as usize) / size_of::<libc::pid_t>();
    pids.into_iter()
        .take(held.min(CHILDREN))
        .filter_map(|child| u32::try_from(child).ok())
        .filter(|child| *child != 0)
        .collect()
}

/// The file `pid` is running, or `None` for a process that has gone or is not ours to ask about.
pub(super) fn executable_of(pid: u32) -> Option<String> {
    let mut path = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: the pointer is to `path`, live for this call, and the size handed over is that
    // allocation's own; the kernel writes no more than it is told and answers how many bytes.
    #[allow(unsafe_code)]
    let written = unsafe {
        libc::proc_pidpath(pid as libc::c_int, path.as_mut_ptr().cast::<c_void>(), path.len() as u32)
    };
    if written <= 0 {
        return None;
    }
    path.truncate(written as usize);
    String::from_utf8(path).ok()
}

/// The environment `pid` was started with, from `KERN_PROCARGS2`. Empty for a process that has
/// gone, is not ours to ask about, or is a platform binary, whose environment the kernel withholds.
fn environment_of(pid: u32) -> Vec<String> {
    let mut name = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as c_int];
    let mut size: libc::size_t = 0;
    // SAFETY: a sizing call. `name` and `size` are live for it, and no buffer is handed over, so
    // the kernel writes only how many bytes the reading takes.
    #[allow(unsafe_code)]
    let sized = unsafe {
        libc::sysctl(name.as_mut_ptr(), 3, std::ptr::null_mut(), &mut size, std::ptr::null_mut(), 0)
    };
    if sized != 0 || size == 0 {
        return Vec::new();
    }
    let mut reading = vec![0u8; size];
    // SAFETY: the pointer is to `reading`, live for this call, and `size` is that allocation's own;
    // the kernel writes no more than it is told and answers in `size` how many bytes it wrote.
    #[allow(unsafe_code)]
    let read = unsafe {
        libc::sysctl(name.as_mut_ptr(), 3, reading.as_mut_ptr().cast::<c_void>(), &mut size, std::ptr::null_mut(), 0)
    };
    if read != 0 {
        return Vec::new();
    }
    reading.truncate(size);
    environment_in(&reading)
}

/// The environment in a `KERN_PROCARGS2` reading: `argc`, the executable's path, nul padding,
/// `argc` arguments and then the environment, each ending in a nul, the environment's end an empty one.
fn environment_in(reading: &[u8]) -> Vec<String> {
    let Some((count, rest)) = reading.split_first_chunk::<4>() else {
        return Vec::new();
    };
    let Ok(arguments) = usize::try_from(c_int::from_ne_bytes(*count)) else {
        return Vec::new();
    };
    let Some(after_path) = rest.iter().position(|byte| *byte == 0).map(|end| &rest[end..]) else {
        return Vec::new();
    };
    let Some(start) = after_path.iter().position(|byte| *byte != 0) else {
        return Vec::new();
    };
    let mut strings = after_path[start..].split(|byte| *byte == 0);
    strings.by_ref().take(arguments).for_each(drop);
    strings
        .take_while(|one| !one.is_empty())
        .map(|one| String::from_utf8_lossy(one).into_owned())
        .collect()
}

/// Every socket file descriptor the process holds. Empty on any failure,
/// including a process that has gone.
fn sockets_of(pid: u32) -> Vec<c_int> {
    let mut entries: Vec<proc_fdinfo> = vec![
        proc_fdinfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
        FDS
    ];
    // Asked once with a generous buffer rather than twice with a sizing call:
    // the answer is how many bytes were written, so a process holding more than
    // `FDS` open files is truncated rather than mis-parsed, and a Drone's
    // session connection is not the two-hundred-and-fifty-seventh thing the CLI
    // opened.
    // SAFETY: the pointer is to `entries`, which is live for this call and
    // longer, and the size handed over is that allocation's own, from
    // `size_of`. The kernel writes no more than it is told, retains nothing,
    // and answers how many bytes it wrote. A pid that has gone is an error
    // return rather than a fault.
    #[allow(unsafe_code)]
    let bytes = unsafe {
        proc_pidinfo(
            pid as c_int,
            LIST_FDS,
            0,
            entries.as_mut_ptr().cast::<c_void>(),
            (size_of::<proc_fdinfo>() * FDS) as c_int,
        )
    };
    if bytes <= 0 {
        return Vec::new();
    }
    let held = (bytes as usize) / size_of::<proc_fdinfo>();
    entries
        .into_iter()
        .take(held.min(FDS))
        .filter(|entry| entry.proc_fdtype == FD_SOCKET)
        .map(|entry| entry.proc_fd)
        .collect()
}

/// The local and foreign ports of one socket, where it is an IP or TCP socket.
///
/// `None` for a unix socket, a kernel control, a descriptor that closed between
/// the list and this call, or any error at all.
fn ports_of(pid: u32, fd: c_int) -> Option<(u16, u16)> {
    let mut buffer = [0u8; BUFFER];
    // SAFETY: `buffer` is a live local and `BUFFER` is its own length. The call
    // refuses a buffer smaller than the flavour's record and copies out that
    // record and no more, so it cannot write past the end; a descriptor closed
    // between the list and here is an error return.
    #[allow(unsafe_code)]
    let bytes = unsafe {
        proc_pidfdinfo(
            pid as c_int,
            fd,
            FD_SOCKET_INFO,
            buffer.as_mut_ptr().cast::<c_void>(),
            BUFFER as c_int,
        )
    };
    if (bytes as usize) < size_of::<SocketPorts>() {
        return None;
    }
    // `read_unaligned` because the buffer is a byte array: the layout is the
    // C one and the compiler has agreed its size above, but the array itself
    // carries no alignment promise.
    // SAFETY: the layout is `#[repr(C)]` and matches `struct socket_fdinfo`'s
    // prefix, whose size the assertions above hold; the guard on `bytes` proves
    // the kernel wrote at least that many. `read_unaligned` and not a deref
    // because a byte array carries no alignment promise. Nothing borrows — the
    // two values taken out are `u16`s copied by value.
    #[allow(unsafe_code)]
    let info = unsafe { buffer.as_ptr().cast::<SocketPorts>().read_unaligned() };
    if info.soi_kind != SOCKINFO_IN && info.soi_kind != SOCKINFO_TCP {
        return None;
    }
    // The kernel keeps these in network byte order in a host-sized field. Both
    // are read the same way, because reading one of them differently is how a
    // pair match becomes a coincidence.
    Some((
        u16::from_be(info.insi_lport as u16),
        u16::from_be(info.insi_fport as u16),
    ))
}
