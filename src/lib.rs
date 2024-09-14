#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code
)]

use std::{
    ffi::{CStr, OsStr, OsString},
    fmt::{Display, Formatter},
    mem::MaybeUninit,
    os::windows::ffi::OsStrExt,
    path::Path,
    ptr, time,
};

use libc::{c_int, c_void, size_t};
use windows::Win32::{
    Security::SECURITY_ATTRIBUTES,
    System::Threading::{PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW},
};

macro_rules! widen {
    ( $str:ident ) => {{
        let mut vector: Vec<u16> = $str.encode_utf16().collect();
        // push a null terminator
        vector.push(0x00);
        vector.as_ptr()
    }};
}

// USVFS Bindings

/// if set, linking fails in case of an error
pub const LINKFLAG_FAILIFEXISTS: u32 = 0x00000001;

/// if set, changes to the source directory after the link operation
/// will be updated in the virtual fs. only relevant in static
/// link directory operations
pub const LINKFLAG_MONITORCHANGES: u32 = 0x00000002;

/// if set, file creation (including move or copy) operations to
/// destination will be redirected to the source. Only one createtarget
/// can be set for a destination folder so this flag will replace
/// the previous create target.
/// If there different create-target have been set for an element and one of its
/// ancestors, the inner-most create-target is used
pub const LINKFLAG_CREATETARGET: u32 = 0x00000004;

/// if set, directories are linked recursively
pub const LINKFLAG_RECURSIVE: u32 = 0x00000008;

/// if set, linking fails if the file or directory is skipped
/// files or directories are skipped depending on whats been added to
/// the skip file suffixes or skip directories list in
/// the sharedparameters class, those lists are checked during virtual linking
pub const LINKFLAG_FAILIFSKIPPED: u32 = 0x00000010;

/// Opaque type for usvfsParameters
#[repr(C)]
pub struct parameters {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl parameters {
    fn new() -> *mut Self {
        unsafe { usvfsCreateParameters() }
    }

    fn set_instance_name(&mut self, name: &str) {
        unsafe { usvfsSetInstanceName(self, name.as_ptr()) }
    }

    fn set_debug_mode(&mut self, debug_mode: bool) {
        unsafe { usvfsSetDebugMode(self, debug_mode) }
    }

    fn set_log_level(&mut self, log_level: LogLevel) {
        unsafe { usvfsSetLogLevel(self, log_level) }
    }

    fn set_crash_dumps_type(&mut self, dump_type: CrashDumpsType) {
        unsafe { usvfsSetCrashDumpType(self, dump_type) }
    }

    fn set_crash_dumps_path(&mut self, path: &str) {
        unsafe { usvfsSetCrashDumpPath(self, path.as_ptr()) }
    }

    fn set_process_delay(&mut self, time: time::Duration) {
        unsafe {
            usvfsSetProcessDelay(
                self,
                time.as_millis()
                    .try_into()
                    .expect("Failed to convert time to milliseconds"),
            )
        };
    }
}

fn create_vfs(params: &parameters) -> Result<(), ()> {
    unsafe { match usvfsCreateVFS(params) {
        true => Ok(()),
        false => Err(()),
    }}
}

fn connect_vfs(params: &parameters) -> Result<(), ()> {
    unsafe { match usvfsConnectVfs(params) {
        true => Ok(()),
        false => Err(()),
    }}
}

fn disconnect_vfs() {
    unsafe { usvfsDisconnectVFS() }
}

fn clear_virtual_mappings() {
    unsafe { usvfsClearVirtualMappings() };
}

fn virtually_link_file(source: &str, destination: &str, flags: u32) -> Result<(), ()> {
    unsafe { match usvfsVirtualLinkFile(widen!(source), widen!(destination), flags) {
        true => Ok(()),
        false => Err(()),
    }}
}

fn virtually_link_directory_static(source: &str, destination: &str, flags: u32) -> Result<(), ()> {
    unsafe { match usvfsVirtualLinkDirectoryStatic(widen!(source), widen!(destination), flags) {
        true => Ok(()),
        false => Err(()),
    }}
}

// fn usvfsGetCurrentVFSName(buffer: *mut u8, size: size_t);
// fn usvfsGetVFSProcessList(count: *mut size_t, processIDs: *mut u32) -> bool;
// fn usvfsGetVFSProcessList2(cont: *mut size_t, buffer: *mut *mut u32) -> bool;

fn create_process_hooked(
    application_name: &str,
    command_line: &str,
    process_attributes: &mut SECURITY_ATTRIBUTES,
    thread_attributes: &mut SECURITY_ATTRIBUTES,
    inherit_handles: bool,
    current_dir: &str,
    startup_information: &mut STARTUPINFOW,
    process_information: &mut PROCESS_INFORMATION,
) -> bool {
    unsafe {
        usvfsCreateProcessHooked(
            widen!(application_name),
            widen!(command_line).cast_mut(),
            process_attributes,
            thread_attributes,
            inherit_handles,
            0,
            ptr::null_mut(),
            widen!(current_dir),
            startup_information,
            process_information,
        )
    }
}

fn init_logging(toLocal: bool) {
    unsafe { usvfsInitLogging(toLocal) }
}

fn get_log_message(dst: &mut [u8], blocking: bool) {
    unsafe {
        // TODO this bool should cause error handeling and return some kind of result
        _ = usvfsGetLogMessage(dst.as_mut_ptr(), &mut dst.len(), blocking)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let strLogLevel = CStr::from_ptr(usvfsLogLevelToString(*self))
                .to_str()
                .expect("Invalid Utf8");
            write!(f, "{}", strLogLevel)
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub enum CrashDumpsType {
    Nil,
    Mini,
    Data,
    Full,
}

impl Display for CrashDumpsType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let strLogLevel = CStr::from_ptr(usvfsCrashDumpTypeToString(*self))
                .to_str()
                .expect("Invalid Utf8");
            write!(f, "{}", strLogLevel)
        }
    }
}

// TODO docs
#[link(name = "usvfs_x64")]
extern "C" {
    fn usvfsCreateParameters() -> *mut parameters;
    fn usvfsDupeParameters(p: *mut parameters) -> *mut parameters;
    fn usvfsCopyParameters(source: *const parameters, dest: *mut parameters);
    fn usvfsFreeParameters(p: *mut parameters);
    fn usvfsSetInstanceName(p: *mut parameters, name: *const u8);
    fn usvfsSetDebugMode(p: *mut parameters, debugMode: bool);
    fn usvfsSetLogLevel(p: *mut parameters, level: LogLevel);
    fn usvfsSetCrashDumpType(p: *mut parameters, dumpType: CrashDumpsType);
    fn usvfsSetCrashDumpPath(p: *mut parameters, path: *const u8);
    fn usvfsSetProcessDelay(p: *mut parameters, milliseconds: c_int);

    fn usvfsLogLevelToString(lv: LogLevel) -> *const i8;
    fn usvfsCrashDumpTypeToString(t: CrashDumpsType) -> *const i8;

    fn usvfsClearVirtualMappings();
    fn usvfsVirtualLinkFile(source: *const u16, destination: *const u16, flags: u32) -> bool;
    fn usvfsVirtualLinkDirectoryStatic(
        source: *const u16,
        destination: *const u16,
        flags: u32,
    ) -> bool;
    fn usvfsConnectVfs(p: *const parameters) -> bool;
    fn usvfsCreateVFS(p: *const parameters) -> bool;
    fn usvfsDisconnectVFS();
    fn usvfsGetCurrentVFSName(buffer: *mut u8, size: size_t);
    fn usvfsGetVFSProcessList(count: *mut size_t, processIDs: *mut u32) -> bool;
    fn usvfsGetVFSProcessList2(cont: *mut size_t, buffer: *mut *mut u32) -> bool;
    fn usvfsCreateProcessHooked(
        lpApplicationName: *const u16,
        lpCommandLine: *mut u16,
        lpProcessAttributes: *mut SECURITY_ATTRIBUTES,
        lpThreadAttributes: *mut SECURITY_ATTRIBUTES,
        bInheritHandles: bool,
        dwCreationFlags: u32,
        lpEnvironment: *mut c_void,
        lpCurrentDirectory: *const u16,
        lpStartupInfo: *mut STARTUPINFOW,
        lpProcessInformation: *mut PROCESS_INFORMATION,
    ) -> bool;
    fn usvfsGetLogMessage(buffer: *mut u8, size: &mut size_t, blocking: bool) -> bool;
    fn usvfsCreateVFSDump(buffer: *mut u8, size: *mut size_t) -> bool;
    fn usvfsBlacklistExecutable(executableName: *mut u16);
    fn usvfsClearExecutableBlacklist();
    fn usvfsAddSkipFileSuffix(fileSuffix: *mut u16);
    fn usvfsClearSkipFileSuffixes();
    fn usvfsAddSkipDirectory(directory: *mut u16);
    fn usvfsClearSkipDirectories();
    fn usvfsForceLoadLibrary(processName: *mut u16, libraryPath: *mut u16);
    fn usvfsClearLibraryForceLoads();
    fn usvfsPrintDebugInfo();
    fn usvfsInitLogging(toLocal: bool);

    fn usvfsUpdateParameters(p: *mut parameters);
    fn usvfsVersionString() -> *mut u8;
}
