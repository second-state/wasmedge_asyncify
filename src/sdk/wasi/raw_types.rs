// bindgen --size_t-is-usize --no-layout-tests --distrust-clang-mangling --no-doc-comments  --allowlist-type="__wasi.*" --default-enum-style rust WasmEdge/thirdparty/wasi/api.hpp -o wasi_types.rs
/* automatically generated by rust-bindgen 0.60.1 */

pub type __uint8_t = ::std::os::raw::c_uchar;
pub type __uint16_t = ::std::os::raw::c_ushort;
pub type __int32_t = ::std::os::raw::c_int;
pub type __uint32_t = ::std::os::raw::c_uint;
pub type __int64_t = ::std::os::raw::c_long;
pub type __uint64_t = ::std::os::raw::c_ulong;
pub type const_uint8_t_ptr = u32;
pub type uint8_t_ptr = u32;
pub type __wasi_size_t = u32;
pub type __wasi_filesize_t = u64;
pub type __wasi_timestamp_t = u64;
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_clockid_t {
    __WASI_CLOCKID_REALTIME = 0,
    __WASI_CLOCKID_MONOTONIC = 1,
    __WASI_CLOCKID_PROCESS_CPUTIME_ID = 2,
    __WASI_CLOCKID_THREAD_CPUTIME_ID = 3,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_errno_t {
    __WASI_ERRNO_SUCCESS = 0,
    __WASI_ERRNO_2BIG = 1,
    __WASI_ERRNO_ACCES = 2,
    __WASI_ERRNO_ADDRINUSE = 3,
    __WASI_ERRNO_ADDRNOTAVAIL = 4,
    __WASI_ERRNO_AFNOSUPPORT = 5,
    __WASI_ERRNO_AGAIN = 6,
    __WASI_ERRNO_ALREADY = 7,
    __WASI_ERRNO_BADF = 8,
    __WASI_ERRNO_BADMSG = 9,
    __WASI_ERRNO_BUSY = 10,
    __WASI_ERRNO_CANCELED = 11,
    __WASI_ERRNO_CHILD = 12,
    __WASI_ERRNO_CONNABORTED = 13,
    __WASI_ERRNO_CONNREFUSED = 14,
    __WASI_ERRNO_CONNRESET = 15,
    __WASI_ERRNO_DEADLK = 16,
    __WASI_ERRNO_DESTADDRREQ = 17,
    __WASI_ERRNO_DOM = 18,
    __WASI_ERRNO_DQUOT = 19,
    __WASI_ERRNO_EXIST = 20,
    __WASI_ERRNO_FAULT = 21,
    __WASI_ERRNO_FBIG = 22,
    __WASI_ERRNO_HOSTUNREACH = 23,
    __WASI_ERRNO_IDRM = 24,
    __WASI_ERRNO_ILSEQ = 25,
    __WASI_ERRNO_INPROGRESS = 26,
    __WASI_ERRNO_INTR = 27,
    __WASI_ERRNO_INVAL = 28,
    __WASI_ERRNO_IO = 29,
    __WASI_ERRNO_ISCONN = 30,
    __WASI_ERRNO_ISDIR = 31,
    __WASI_ERRNO_LOOP = 32,
    __WASI_ERRNO_MFILE = 33,
    __WASI_ERRNO_MLINK = 34,
    __WASI_ERRNO_MSGSIZE = 35,
    __WASI_ERRNO_MULTIHOP = 36,
    __WASI_ERRNO_NAMETOOLONG = 37,
    __WASI_ERRNO_NETDOWN = 38,
    __WASI_ERRNO_NETRESET = 39,
    __WASI_ERRNO_NETUNREACH = 40,
    __WASI_ERRNO_NFILE = 41,
    __WASI_ERRNO_NOBUFS = 42,
    __WASI_ERRNO_NODEV = 43,
    __WASI_ERRNO_NOENT = 44,
    __WASI_ERRNO_NOEXEC = 45,
    __WASI_ERRNO_NOLCK = 46,
    __WASI_ERRNO_NOLINK = 47,
    __WASI_ERRNO_NOMEM = 48,
    __WASI_ERRNO_NOMSG = 49,
    __WASI_ERRNO_NOPROTOOPT = 50,
    __WASI_ERRNO_NOSPC = 51,
    __WASI_ERRNO_NOSYS = 52,
    __WASI_ERRNO_NOTCONN = 53,
    __WASI_ERRNO_NOTDIR = 54,
    __WASI_ERRNO_NOTEMPTY = 55,
    __WASI_ERRNO_NOTRECOVERABLE = 56,
    __WASI_ERRNO_NOTSOCK = 57,
    __WASI_ERRNO_NOTSUP = 58,
    __WASI_ERRNO_NOTTY = 59,
    __WASI_ERRNO_NXIO = 60,
    __WASI_ERRNO_OVERFLOW = 61,
    __WASI_ERRNO_OWNERDEAD = 62,
    __WASI_ERRNO_PERM = 63,
    __WASI_ERRNO_PIPE = 64,
    __WASI_ERRNO_PROTO = 65,
    __WASI_ERRNO_PROTONOSUPPORT = 66,
    __WASI_ERRNO_PROTOTYPE = 67,
    __WASI_ERRNO_RANGE = 68,
    __WASI_ERRNO_ROFS = 69,
    __WASI_ERRNO_SPIPE = 70,
    __WASI_ERRNO_SRCH = 71,
    __WASI_ERRNO_STALE = 72,
    __WASI_ERRNO_TIMEDOUT = 73,
    __WASI_ERRNO_TXTBSY = 74,
    __WASI_ERRNO_XDEV = 75,
    __WASI_ERRNO_NOTCAPABLE = 76,
    __WASI_ERRNO_AIADDRFAMILY = 77,
    __WASI_ERRNO_AIAGAIN = 78,
    __WASI_ERRNO_AIBADFLAG = 79,
    __WASI_ERRNO_AIFAIL = 80,
    __WASI_ERRNO_AIFAMILY = 81,
    __WASI_ERRNO_AIMEMORY = 82,
    __WASI_ERRNO_AINODATA = 83,
    __WASI_ERRNO_AINONAME = 84,
    __WASI_ERRNO_AISERVICE = 85,
    __WASI_ERRNO_AISOCKTYPE = 86,
    __WASI_ERRNO_AISYSTEM = 87,
}
#[repr(u64)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_rights_t {
    __WASI_RIGHTS_FD_DATASYNC = 1,
    __WASI_RIGHTS_FD_READ = 2,
    __WASI_RIGHTS_FD_SEEK = 4,
    __WASI_RIGHTS_FD_FDSTAT_SET_FLAGS = 8,
    __WASI_RIGHTS_FD_SYNC = 16,
    __WASI_RIGHTS_FD_TELL = 32,
    __WASI_RIGHTS_FD_WRITE = 64,
    __WASI_RIGHTS_FD_ADVISE = 128,
    __WASI_RIGHTS_FD_ALLOCATE = 256,
    __WASI_RIGHTS_PATH_CREATE_DIRECTORY = 512,
    __WASI_RIGHTS_PATH_CREATE_FILE = 1024,
    __WASI_RIGHTS_PATH_LINK_SOURCE = 2048,
    __WASI_RIGHTS_PATH_LINK_TARGET = 4096,
    __WASI_RIGHTS_PATH_OPEN = 8192,
    __WASI_RIGHTS_FD_READDIR = 16384,
    __WASI_RIGHTS_PATH_READLINK = 32768,
    __WASI_RIGHTS_PATH_RENAME_SOURCE = 65536,
    __WASI_RIGHTS_PATH_RENAME_TARGET = 131072,
    __WASI_RIGHTS_PATH_FILESTAT_GET = 262144,
    __WASI_RIGHTS_PATH_FILESTAT_SET_SIZE = 524288,
    __WASI_RIGHTS_PATH_FILESTAT_SET_TIMES = 1048576,
    __WASI_RIGHTS_FD_FILESTAT_GET = 2097152,
    __WASI_RIGHTS_FD_FILESTAT_SET_SIZE = 4194304,
    __WASI_RIGHTS_FD_FILESTAT_SET_TIMES = 8388608,
    __WASI_RIGHTS_PATH_SYMLINK = 16777216,
    __WASI_RIGHTS_PATH_REMOVE_DIRECTORY = 33554432,
    __WASI_RIGHTS_PATH_UNLINK_FILE = 67108864,
    __WASI_RIGHTS_POLL_FD_READWRITE = 134217728,
    __WASI_RIGHTS_SOCK_SHUTDOWN = 268435456,
    __WASI_RIGHTS_SOCK_OPEN = 536870912,
    __WASI_RIGHTS_SOCK_CLOSE = 1073741824,
    __WASI_RIGHTS_SOCK_BIND = 2147483648,
    __WASI_RIGHTS_SOCK_RECV = 4294967296,
    __WASI_RIGHTS_SOCK_RECV_FROM = 8589934592,
    __WASI_RIGHTS_SOCK_SEND = 17179869184,
    __WASI_RIGHTS_SOCK_SEND_TO = 34359738368,
}
pub type __wasi_fd_t = i32;
pub type __wasi_sock_d_t = __wasi_fd_t;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_iovec_t {
    pub buf: uint8_t_ptr,
    pub buf_len: __wasi_size_t,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_ciovec_t {
    pub buf: const_uint8_t_ptr,
    pub buf_len: __wasi_size_t,
}
pub type __wasi_filedelta_t = i64;
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_whence_t {
    __WASI_WHENCE_SET = 0,
    __WASI_WHENCE_CUR = 1,
    __WASI_WHENCE_END = 2,
}
pub type __wasi_dircookie_t = u64;
pub type __wasi_dirnamlen_t = u32;
pub type __wasi_inode_t = u64;
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_filetype_t {
    __WASI_FILETYPE_UNKNOWN = 0,
    __WASI_FILETYPE_BLOCK_DEVICE = 1,
    __WASI_FILETYPE_CHARACTER_DEVICE = 2,
    __WASI_FILETYPE_DIRECTORY = 3,
    __WASI_FILETYPE_REGULAR_FILE = 4,
    __WASI_FILETYPE_SOCKET_DGRAM = 5,
    __WASI_FILETYPE_SOCKET_STREAM = 6,
    __WASI_FILETYPE_SYMBOLIC_LINK = 7,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_dirent_t {
    pub d_next: __wasi_dircookie_t,
    pub d_ino: __wasi_inode_t,
    pub d_namlen: __wasi_dirnamlen_t,
    pub d_type: __wasi_filetype_t,
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_advice_t {
    __WASI_ADVICE_NORMAL = 0,
    __WASI_ADVICE_SEQUENTIAL = 1,
    __WASI_ADVICE_RANDOM = 2,
    __WASI_ADVICE_WILLNEED = 3,
    __WASI_ADVICE_DONTNEED = 4,
    __WASI_ADVICE_NOREUSE = 5,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_fdflags_t {
    __WASI_FDFLAGS_APPEND = 1,
    __WASI_FDFLAGS_DSYNC = 2,
    __WASI_FDFLAGS_NONBLOCK = 4,
    __WASI_FDFLAGS_RSYNC = 8,
    __WASI_FDFLAGS_SYNC = 16,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_fdstat_t {
    pub fs_filetype: __wasi_filetype_t,
    pub fs_flags: __wasi_fdflags_t,
    pub fs_rights_base: __wasi_rights_t,
    pub fs_rights_inheriting: __wasi_rights_t,
}
pub type __wasi_device_t = u64;
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_fstflags_t {
    __WASI_FSTFLAGS_ATIM = 1,
    __WASI_FSTFLAGS_ATIM_NOW = 2,
    __WASI_FSTFLAGS_MTIM = 4,
    __WASI_FSTFLAGS_MTIM_NOW = 8,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_lookupflags_t {
    __WASI_LOOKUPFLAGS_SYMLINK_FOLLOW = 1,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_oflags_t {
    __WASI_OFLAGS_CREAT = 1,
    __WASI_OFLAGS_DIRECTORY = 2,
    __WASI_OFLAGS_EXCL = 4,
    __WASI_OFLAGS_TRUNC = 8,
}
pub type __wasi_linkcount_t = u64;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_filestat_t {
    pub dev: __wasi_device_t,
    pub ino: __wasi_inode_t,
    pub filetype: __wasi_filetype_t,
    pub nlink: __wasi_linkcount_t,
    pub size: __wasi_filesize_t,
    pub atim: __wasi_timestamp_t,
    pub mtim: __wasi_timestamp_t,
    pub ctim: __wasi_timestamp_t,
}
pub type __wasi_userdata_t = u64;
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_eventtype_t {
    __WASI_EVENTTYPE_CLOCK = 0,
    __WASI_EVENTTYPE_FD_READ = 1,
    __WASI_EVENTTYPE_FD_WRITE = 2,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_eventrwflags_t {
    __WASI_EVENTRWFLAGS_FD_READWRITE_NONE = 0,
    __WASI_EVENTRWFLAGS_FD_READWRITE_HANGUP = 1,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_event_fd_readwrite_t {
    pub nbytes: __wasi_filesize_t,
    pub flags: __wasi_eventrwflags_t,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_event_t {
    pub userdata: __wasi_userdata_t,
    pub error: __wasi_errno_t,
    pub type_: __wasi_eventtype_t,
    pub fd_readwrite: __wasi_event_fd_readwrite_t,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_subclockflags_t {
    __WASI_SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME_NO_SET = 0,
    __WASI_SUBCLOCKFLAGS_SUBSCRIPTION_CLOCK_ABSTIME_SET = 1,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_subscription_clock_t {
    pub id: __wasi_clockid_t,
    pub timeout: __wasi_timestamp_t,
    pub precision: __wasi_timestamp_t,
    pub flags: __wasi_subclockflags_t,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_subscription_fd_readwrite_t {
    pub file_descriptor: __wasi_fd_t,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union __wasi_subscription_u_u_t {
    pub clock: __wasi_subscription_clock_t,
    pub fd_read: __wasi_subscription_fd_readwrite_t,
    pub fd_write: __wasi_subscription_fd_readwrite_t,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __wasi_subscription_u_t {
    pub tag: __wasi_eventtype_t,
    pub u: __wasi_subscription_u_u_t,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __wasi_subscription_t {
    pub userdata: __wasi_userdata_t,
    pub u: __wasi_subscription_u_t,
}
pub type __wasi_exitcode_t = u32;
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_signal_t {
    __WASI_SIGNAL_NONE = 0,
    __WASI_SIGNAL_HUP = 1,
    __WASI_SIGNAL_INT = 2,
    __WASI_SIGNAL_QUIT = 3,
    __WASI_SIGNAL_ILL = 4,
    __WASI_SIGNAL_TRAP = 5,
    __WASI_SIGNAL_ABRT = 6,
    __WASI_SIGNAL_BUS = 7,
    __WASI_SIGNAL_FPE = 8,
    __WASI_SIGNAL_KILL = 9,
    __WASI_SIGNAL_USR1 = 10,
    __WASI_SIGNAL_SEGV = 11,
    __WASI_SIGNAL_USR2 = 12,
    __WASI_SIGNAL_PIPE = 13,
    __WASI_SIGNAL_ALRM = 14,
    __WASI_SIGNAL_TERM = 15,
    __WASI_SIGNAL_CHLD = 16,
    __WASI_SIGNAL_CONT = 17,
    __WASI_SIGNAL_STOP = 18,
    __WASI_SIGNAL_TSTP = 19,
    __WASI_SIGNAL_TTIN = 20,
    __WASI_SIGNAL_TTOU = 21,
    __WASI_SIGNAL_URG = 22,
    __WASI_SIGNAL_XCPU = 23,
    __WASI_SIGNAL_XFSZ = 24,
    __WASI_SIGNAL_VTALRM = 25,
    __WASI_SIGNAL_PROF = 26,
    __WASI_SIGNAL_WINCH = 27,
    __WASI_SIGNAL_POLL = 28,
    __WASI_SIGNAL_PWR = 29,
    __WASI_SIGNAL_SYS = 30,
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_address_family_t {
    __WASI_ADDRESS_FAMILY_UNSPEC = 0,
    __WASI_ADDRESS_FAMILY_INET4 = 1,
    __WASI_ADDRESS_FAMILY_INET6 = 2,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_address_t {
    pub buf: uint8_t_ptr,
    pub buf_len: __wasi_size_t,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_sock_opt_level_t {
    __WASI_SOCK_OPT_LEVEL_SOL_SOCKET = 0,
}
#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_sock_opt_so_t {
    __WASI_SOCK_OPT_SO_REUSEADDR = 0,
    __WASI_SOCK_OPT_SO_TYPE = 1,
    __WASI_SOCK_OPT_SO_ERROR = 2,
    __WASI_SOCK_OPT_SO_DONTROUTE = 3,
    __WASI_SOCK_OPT_SO_BROADCAST = 4,
    __WASI_SOCK_OPT_SO_SNDBUF = 5,
    __WASI_SOCK_OPT_SO_RCVBUF = 6,
    __WASI_SOCK_OPT_SO_KEEPALIVE = 7,
    __WASI_SOCK_OPT_SO_OOBINLINE = 8,
    __WASI_SOCK_OPT_SO_LINGER = 9,
    __WASI_SOCK_OPT_SO_RCVLOWAT = 10,
    __WASI_SOCK_OPT_SO_RCVTIMEO = 11,
    __WASI_SOCK_OPT_SO_SNDTIMEO = 12,
    __WASI_SOCK_OPT_SO_ACCEPTCONN = 13,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_aiflags_t {
    __WASI_AIFLAGS_AI_PASSIVE = 1,
    __WASI_AIFLAGS_AI_CANONNAME = 2,
    __WASI_AIFLAGS_AI_NUMERICHOST = 4,
    __WASI_AIFLAGS_AI_NUMERICSERV = 8,
    __WASI_AIFLAGS_AI_V4MAPPED = 16,
    __WASI_AIFLAGS_AI_ALL = 32,
    __WASI_AIFLAGS_AI_ADDRCONFIG = 64,
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_sock_type_t {
    __WASI_SOCK_TYPE_SOCK_ANY = 0,
    __WASI_SOCK_TYPE_SOCK_DGRAM = 1,
    __WASI_SOCK_TYPE_SOCK_STREAM = 2,
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_protocol_t {
    __WASI_PROTOCOL_IPPROTO_IP = 0,
    __WASI_PROTOCOL_IPPROTO_TCP = 1,
    __WASI_PROTOCOL_IPPROTO_UDP = 2,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_sockaddr_in_t {
    pub sin_family: __wasi_address_family_t,
    pub sin_port: u16,
    pub sin_addr: __wasi_address_t,
    pub sin_zero_len: __wasi_size_t,
    pub sin_zero: uint8_t_ptr,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_sockaddr_t {
    pub sa_family: __wasi_address_family_t,
    pub sa_data_len: __wasi_size_t,
    pub sa_data: uint8_t_ptr,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_addrinfo_t {
    pub ai_flags: __wasi_aiflags_t,
    pub ai_family: __wasi_address_family_t,
    pub ai_socktype: __wasi_sock_type_t,
    pub ai_protocol: __wasi_protocol_t,
    pub ai_addrlen: __wasi_size_t,
    pub ai_addr: uint8_t_ptr,
    pub ai_canonname: uint8_t_ptr,
    pub ai_canonname_len: __wasi_size_t,
    pub ai_next: uint8_t_ptr,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_riflags_t {
    __WASI_RIFLAGS_RECV_PEEK = 1,
    __WASI_RIFLAGS_RECV_WAITALL = 2,
}
#[repr(u16)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_roflags_t {
    __WASI_ROFLAGS_RECV_DATA_TRUNCATED = 1,
}
pub type __wasi_siflags_t = u16;
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_sdflags_t {
    __WASI_SDFLAGS_RD = 1,
    __WASI_SDFLAGS_WR = 2,
}
#[repr(u8)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum __wasi_preopentype_t {
    __WASI_PREOPENTYPE_DIR = 0,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct __wasi_prestat_dir_t {
    pub pr_name_len: __wasi_size_t,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union __wasi_prestat_u_t {
    pub dir: __wasi_prestat_dir_t,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __wasi_prestat_t {
    pub tag: __wasi_preopentype_t,
    pub u: __wasi_prestat_u_t,
}