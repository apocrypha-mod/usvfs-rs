#![allow(non_snake_case,non_camel_case_types,non_upper_case_globals,dead_code)]

use std::{
    ffi::{OsStr, OsString}, mem::MaybeUninit, os::windows::ffi::OsStrExt, path::Path, ptr, time
};

use libc::{c_void, c_int, size_t};
use windows::Win32::{
    System::Threading::{STARTUPINFOW, PROCESS_INFORMATION, PROCESS_CREATION_FLAGS},
    Security::SECURITY_ATTRIBUTES,
};

macro_rules! widen {
    ( $str:ident ) => {{
        let mut vector: Vec<u16> = $str.encode_utf16().collect();
        vector.push(0);
        vector.as_ptr()
    }};
}

// USVFS Bindings

pub const LINKFLAG_FAILIFEXISTS:   u32 = 0x00000001;
pub const LINKFLAG_MONITORCHANGES: u32 = 0x00000002;
pub const LINKFLAG_CREATETARGET:   u32 = 0x00000004;
pub const LINKFLAG_RECURSIVE:      u32 = 0x00000008;
pub const LINKFLAG_FAILIFSKIPPED:  u32 = 0x00000010;

pub struct Child {
    process_information: PROCESS_INFORMATION,
}

/// Opaque type for usvfsParameters
#[repr(C)] pub struct parameters {
    _data: [u8; 0],
    _marker:
        core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl parameters {
    fn new() -> Box<Self> {
        unsafe { Box::from_raw(usvfsCreateParameters()) }
    }

    fn set_instance_name(&mut self, name: &str) {
        unsafe { usvfsSetInstanceName(self,  name.as_ptr()) }
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
        unsafe { usvfsSetProcessDelay(
            self,
            time.as_millis().try_into().expect("Failed to convert time to milliseconds")
        )};
    }
}

fn create_vfs(params: &crate::parameters) -> bool {
    unsafe { crate::usvfsCreateVFS(params) }
}

fn connect_vfs(params: &crate::parameters) -> bool {
    unsafe { crate::usvfsConnectVfs(params) }
}

fn disconnect_vfs() {
    unsafe { crate::usvfsDisconnectVFS() }
}

fn clear_virtual_mappings() {
    unsafe { crate::usvfsClearVirtualMappings() };
}

fn virtually_link_file(source: &str, destination: &str, flags: u32) -> bool {
    unsafe { crate::usvfsVirtualLinkFile(widen!(source), widen!(destination), flags) }
}

fn virtually_link_directory_static(source: &str, destination: &str, flags: u32) -> bool {
    unsafe { crate::usvfsVirtualLinkDirectoryStatic(widen!(source), widen!(destination), flags) }
}

fn create_process_hooked(
    application_name: &str,
    command_line: &str,
    inherit_handles: bool,
    current_dir: &str,
    startup_information: &mut STARTUPINFOW,
    process_information: &mut PROCESS_INFORMATION,
) -> bool {
    unsafe { crate::usvfsCreateProcessHooked(
        widen!(application_name),
        widen!(command_line).cast_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
        inherit_handles,
        0,
        ptr::null_mut(),
        widen!(current_dir),
        startup_information,
        process_information,
    ) }
}

fn init_logging(toLocal: bool) {
    unsafe { crate::usvfsInitLogging(toLocal) }
}

// TODO figure this out
fn get_log_message() -> String {
    //unsafe {
    //    let mut buffer: *mut u8 = "".as_mut_ptr();
    //    let mut size: size_t = 0;
    //    crate::usvfsGetLogMessage(
    //        buffer,
    //        &mut size,
    //        false
    //    );
    //    String::from_raw_parts(buffer, size, size)
    //}
    String::from("")
}

#[repr(C)] pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error
}

// impl Display for LogLevel {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.pad(unsafe {
//             usvfsLogLevelToString(*self).as_ref()
//         })
//     }
// }

#[repr(C)] pub enum CrashDumpsType {
    Nil,
    Mini,
    Data,
    Full
}

// For docs check include/usvfs.h
// TODO Ill copy it over at some point
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

    fn usvfsLogLevelToString(lv: LogLevel) -> *const u8;
    fn usvfsCrashDumpTypeToString(t: CrashDumpsType) -> *const u8;

    fn usvfsClearVirtualMappings();
    fn usvfsVirtualLinkFile(source: *const u16, destination: *const u16, flags: u32) -> bool;
    fn usvfsVirtualLinkDirectoryStatic(source: *const u16, destination: *const u16, flags: u32) -> bool;
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
