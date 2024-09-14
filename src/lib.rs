#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code
)]
#![feature(arbitrary_self_types_pointers)]

use std::{
    ffi::CStr,
    fmt::{Display, Formatter},
    ptr, time,
};

use libc::{c_int, c_void, size_t};
use windows::Win32::{
    Security::SECURITY_ATTRIBUTES,
    System::Threading::{PROCESS_INFORMATION, STARTUPINFOW},
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
pub struct Parameters {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl Parameters {
    fn new() -> *mut Self {
        unsafe { usvfsCreateParameters() }
    }

    fn set_instance_name(self: *mut Parameters, name: &str) {
        unsafe { usvfsSetInstanceName(self, name.as_ptr()) }
    }

    fn set_debug_mode(self: *mut Parameters, debug_mode: bool) {
        unsafe { usvfsSetDebugMode(self, debug_mode) }
    }

    fn set_log_level(self: *mut Parameters, log_level: LogLevel) {
        unsafe { usvfsSetLogLevel(self, log_level) }
    }

    fn set_crash_dumps_type(self: *mut Parameters, dump_type: CrashDumpsType) {
        unsafe { usvfsSetCrashDumpType(self, dump_type) }
    }

    fn set_crash_dumps_path(self: *mut Parameters, path: &str) {
        unsafe { usvfsSetCrashDumpPath(self, path.as_ptr()) }
    }

    fn set_process_delay(self: *mut Parameters, time: time::Duration) {
        unsafe {
            usvfsSetProcessDelay(
                self,
                time.as_millis()
                    .try_into()
                    .expect("Failed to convert time to milliseconds"),
            )
        };
    }

    fn free_parameters(self: *mut Parameters) {
        unsafe { usvfsFreeParameters(self) }
    }
}

pub fn create_vfs(params: *const Parameters) -> Result<(), ()> {
    unsafe {
        match usvfsCreateVFS(params) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

pub fn connect_vfs(params: *const Parameters) -> Result<(), ()> {
    unsafe {
        match usvfsConnectVfs(params) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

pub fn disconnect_vfs() {
    unsafe { usvfsDisconnectVFS() }
}

pub fn clear_virtual_mappings() {
    unsafe { usvfsClearVirtualMappings() };
}

pub fn virtually_link_file(source: &str, destination: &str, flags: u32) -> Result<(), ()> {
    unsafe {
        match usvfsVirtualLinkFile(widen!(source), widen!(destination), flags) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

pub fn virtually_link_directory_static(
    source: &str,
    destination: &str,
    flags: u32,
) -> Result<(), ()> {
    unsafe {
        match usvfsVirtualLinkDirectoryStatic(widen!(source), widen!(destination), flags) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

pub fn get_current_VFS_name(buffer: &mut [u8]) {
    unsafe { usvfsGetCurrentVFSName(buffer.as_mut_ptr(), buffer.len()) }
}

pub fn create_process_hooked(
    application_name: &str,
    command_line: &str,
    process_attributes: &mut SECURITY_ATTRIBUTES,
    thread_attributes: &mut SECURITY_ATTRIBUTES,
    inherit_handles: bool,
    current_dir: &str,
    startup_information: &mut STARTUPINFOW,
    process_information: &mut PROCESS_INFORMATION,
) -> Result<(), ()> {
    unsafe {
        match usvfsCreateProcessHooked(
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
        ) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

pub fn init_logging(toLocal: bool) {
    unsafe { usvfsInitLogging(toLocal) }
}

pub fn get_log_message(dst: &mut [u8], blocking: bool) {
    unsafe {
        // TODO this bool should cause error handeling and return some kind of result
        _ = usvfsGetLogMessage(dst.as_mut_ptr(), &mut dst.len(), blocking)
    }
}

pub fn create_vfs_dump(buffer: &mut [u8]) -> Result<(), ()> {
    unsafe {
        match usvfsCreateVFSDump(buffer.as_mut_ptr(), &mut buffer.len()) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

fn vec_utf16_from_str(string: &str) -> Vec<u16> {
    let v = string.encode_utf16().collect();
    v
}

pub fn blacklist_executable(executableName: &str) {
    let mut v: Vec<u16> = vec_utf16_from_str(executableName);
    unsafe { usvfsBlacklistExecutable(v.as_mut_ptr() as *mut u16) }
}

pub fn clear_executable_blacklist() {
    unsafe { usvfsClearExecutableBlacklist() }
}

pub fn add_skip_file_suffix(fileSuffix: &str) {
    let mut v: Vec<u16> = vec_utf16_from_str(fileSuffix);
    unsafe { usvfsAddSkipFileSuffix(v.as_mut_ptr() as *mut u16) }
}

pub fn clear_skip_file_suffixes() {
    unsafe { usvfsClearSkipFileSuffixes() }
}

pub fn add_skip_directory(directory: &str) {
    let mut v: Vec<u16> = vec_utf16_from_str(directory);
    unsafe { usvfsAddSkipDirectory(v.as_mut_ptr() as *mut u16) }
}

pub fn clear_skip_directories() {
    unsafe { usvfsClearSkipDirectories() }
}

pub fn force_load_library(processName: &str, libraryPath: &str) {
    let mut nameV = vec_utf16_from_str(processName);
    let mut pathV = vec_utf16_from_str(libraryPath);
    unsafe {
        usvfsForceLoadLibrary(
            nameV.as_mut_ptr() as *mut u16,
            pathV.as_mut_ptr() as *mut u16,
        )
    }
}

pub fn clear_library_force_loads() {
    unsafe { usvfsClearLibraryForceLoads() }
}

fn print_debug_info() {
    unsafe { usvfsPrintDebugInfo() }
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

#[link(name = "usvfs_x64")]
extern "C" {
    fn usvfsCreateParameters() -> *mut Parameters;
    fn usvfsDupeParameters(p: *const Parameters) -> *mut Parameters;
    fn usvfsCopyParameters(source: *const Parameters, dest: *mut Parameters);
    fn usvfsFreeParameters(p: *mut Parameters);
    fn usvfsSetInstanceName(p: *mut Parameters, name: *const u8);
    fn usvfsSetDebugMode(p: *mut Parameters, debugMode: bool);
    fn usvfsSetLogLevel(p: *mut Parameters, level: LogLevel);
    fn usvfsSetCrashDumpType(p: *mut Parameters, dumpType: CrashDumpsType);
    fn usvfsSetCrashDumpPath(p: *mut Parameters, path: *const u8);
    fn usvfsSetProcessDelay(p: *mut Parameters, milliseconds: c_int);

    fn usvfsLogLevelToString(lv: LogLevel) -> *const i8;
    fn usvfsCrashDumpTypeToString(t: CrashDumpsType) -> *const i8;

    fn usvfsClearVirtualMappings();
    fn usvfsVirtualLinkFile(source: *const u16, destination: *const u16, flags: u32) -> bool;
    fn usvfsVirtualLinkDirectoryStatic(
        source: *const u16,
        destination: *const u16,
        flags: u32,
    ) -> bool;
    fn usvfsConnectVfs(p: *const Parameters) -> bool;
    fn usvfsCreateVFS(p: *const Parameters) -> bool;
    fn usvfsDisconnectVFS();
    fn usvfsGetCurrentVFSName(buffer: *mut u8, size: size_t);
    /// unsafe
    pub fn usvfsGetVFSProcessList(count: *mut size_t, processIDs: *mut u32) -> bool;
    /// unsafe
    pub fn usvfsGetVFSProcessList2(cont: *mut size_t, buffer: *mut *mut u32) -> bool;
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

    fn usvfsUpdateParameters(p: *mut Parameters);
    fn usvfsVersionString() -> *mut u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters() {
        let testParams = Parameters::new();
        testParams.set_instance_name("testInstance");
        testParams.set_debug_mode(false);
        testParams.set_log_level(LogLevel::Debug);
        testParams.set_crash_dumps_type(CrashDumpsType::Full);
        testParams.set_crash_dumps_path("");
        testParams.set_process_delay(time::Duration::new(1, 0));
        testParams.free_parameters();
    }

    #[test]
    fn startAndStop() {
        let testParams = Parameters::new();
        testParams.set_instance_name("test");
        testParams.set_debug_mode(false);
        testParams.set_log_level(LogLevel::Debug);
        testParams.set_crash_dumps_type(CrashDumpsType::Nil);
        testParams.set_crash_dumps_path("");

        init_logging(false);
        create_vfs(testParams).expect("Failed to create VFS");
        disconnect_vfs();
        testParams.free_parameters();
    }
}
