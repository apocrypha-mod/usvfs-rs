#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code
)]
#![feature(arbitrary_self_types_pointers)]

use std::{
    ffi::{CStr, CString},
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

macro_rules! widen_mut {
    ( $str:ident ) => {{
        let mut vector: Vec<u16> = $str.encode_utf16().collect();
        // push a null terminator
        vector.push(0x00);
        vector.as_mut_ptr()
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
/// This type stores information about the VFS to be
/// created. To create a VFS, create a new Parameters
/// and run the set functions on it to set properties
/// since the struct is opaque, it can only be interacted
/// with by these functions. The function free_parameters()
/// **MUST** be run after use since we must tell the C++
/// library to deallocate it which cannot be handled from Rust
#[repr(C)]
pub struct Parameters {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

impl Parameters {
    /// Creates a new Parameters
    fn new() -> *mut Self {
        unsafe { usvfsCreateParameters() }
    }

    /// set the name for the VFS instance
    fn set_instance_name(self: *mut Parameters, name: &str) {
        unsafe {
            let cName = CString::new(name).expect("Invalid C-String");
            usvfsSetInstanceName(self, cName.as_ptr())
        }
    }

    /// set whether the VFS should output debug information
    fn set_debug_mode(self: *mut Parameters, debug_mode: bool) {
        unsafe { usvfsSetDebugMode(self, debug_mode) }
    }

    /// set the VFS log level
    fn set_log_level(self: *mut Parameters, log_level: LogLevel) {
        unsafe { usvfsSetLogLevel(self, log_level) }
    }

    /// set the VFS crash dumps type
    fn set_crash_dumps_type(self: *mut Parameters, dump_type: CrashDumpsType) {
        unsafe { usvfsSetCrashDumpType(self, dump_type) }
    }

    /// set the path for crash dumps. An empty string "" dumps to
    /// the current working directory
    fn set_crash_dumps_path(self: *mut Parameters, path: &str) {
        unsafe {
            let cPath = CString::new(path).expect("Invalid C-String");
            usvfsSetCrashDumpPath(self, cPath.as_ptr())
        }
    }

    /// set the amount of time to delay the process
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

    /// free the parameter's memory in C. Not calling this is a memory leak!!!
    /// Rust's automatic destructor: the drop trait cannot be used on memory that
    /// Rust does not own (ie only has a pointer to), so this is REQUIRED
    /// only free parameters after closing any associated VFSs
    fn free_parameters(self: *mut Parameters) {
        unsafe { usvfsFreeParameters(self) }
    }
}

/// creates a new vfs from a parameters struct. You can think of
/// the VFS as a sperate thread or process which you communicate
/// to with the set of functions here.
///
/// This is similar to ConnectVFS except it guarantees
/// the vfs is reset before use.
///
/// Please note that you can only be connected to one vfs, so this will silently disconnect
/// from a previous vfs.
pub fn create_vfs(params: *const Parameters) -> Result<(), ()> {
    unsafe {
        match usvfsCreateVFS(params) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

/// connect to a virtual filesystem as a controller, without hooking the calling process.
///
/// Please note that you can only be connected to one vfs, so this will silently disconnect
/// from a previous vfs.
pub fn connect_vfs(params: *const Parameters) -> Result<(), ()> {
    unsafe {
        match usvfsConnectVfs(params) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

/// disconnect from a virtual filesystem. This removes hooks if necessary
pub fn disconnect_vfs() {
    unsafe { usvfsDisconnectVFS() }
}

/// removes all virtual mappings
pub fn clear_virtual_mappings() {
    unsafe { usvfsClearVirtualMappings() };
}

/// link a file virtually
/// the directory the destination file resides in has to exist - at least virtually
///
/// Virtual operations:
///   - link file
///   - link directory (empty)
///   - link directory (static)
///   - link directory (dynamic)
///   - delete file
///   - delete directory
/// Maybe:
///   - rename/move (= copy + delete)
///   - copy-on-write semantics (changes to files are done in a separate copy of the file, the original is kept on disc but hidden)
pub fn virtually_link_file(source: &str, destination: &str, flags: u32) -> Result<(), ()> {
    unsafe {
        match usvfsVirtualLinkFile(widen!(source), widen!(destination), flags) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

/// link a directory virtually. This static variant recursively links all files individually, change notifications
/// are used to update the information.
/// failIfExists if true, this call fails if the destination directory exists (virtually or physically)
///
/// Virtual operations:
///   - link file
///   - link directory (empty)
///   - link directory (static)
///   - link directory (dynamic)
///   - delete file
///   - delete directory
/// Maybe:
///   - rename/move (= copy + delete)
///   - copy-on-write semantics (changes to files are done in a separate copy of the file, the original is kept on disc but hidden)
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

/// gets the instance name of the current VFS and places it into buffer
pub fn get_current_VFS_name(buffer: &mut [u8]) {
    unsafe { usvfsGetCurrentVFSName(buffer.as_mut_ptr(), buffer.len()) }
}

/// spawn a new process that can see the virtual file system. The signature is identical to CreateProcess
/// but a bit more rusty. Still requires windows stuff.
/// I will impliment some way to pass these to C as null, since in many cases the user does not
/// care to have these back.
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

/// begin logging on the VFS
pub fn init_logging(toLocal: bool) {
    unsafe { usvfsInitLogging(toLocal) }
}

/// get a single log message
/// not sure if this currently works upstream
/// should take a destination buffer for the log message
/// set blocking to false since true isn't implimented upstream
pub fn get_log_message(dst: &mut [u8], blocking: bool) {
    unsafe {
        // TODO this bool should cause error handeling and return some kind of result
        _ = usvfsGetLogMessage(dst.as_mut_ptr(), &mut dst.len(), blocking)
    }
}

/// retrieves a readable representation of the vfs tree
/// the buffer to write to can be null if you only want to determine the required buffer size
/// size is a pointer to the variable that contains the buffer and is updated to the size on return
/// I'm not sure how exactly this will work from Rust, currently unstable and not tested
pub fn create_vfs_dump(buffer: &mut [u8], size: *mut usize) -> Result<(), ()> {
    unsafe {
        match usvfsCreateVFSDump(buffer.as_mut_ptr(), size) {
            true => Ok(()),
            false => Err(()),
        }
    }
}

/// add an executable to the blacklist so it doesn't get exposed
/// to the virtual file system
pub fn blacklist_executable(executableName: &str) {
    unsafe { usvfsBlacklistExecutable(widen_mut!(executableName)) }
}

/// clears the executable blacklist
pub fn clear_executable_blacklist() {
    unsafe { usvfsClearExecutableBlacklist() }
}

/// adds a file suffix to a list to skip during file linking
/// .txt and some_file.txt are both valid file suffixes,
/// not to be confused with file extensions
pub fn add_skip_file_suffix(fileSuffix: &str) {
    unsafe { usvfsAddSkipFileSuffix(widen_mut!(fileSuffix)) }
}

/// clears the file suffix skip-list
pub fn clear_skip_file_suffixes() {
    unsafe { usvfsClearSkipFileSuffixes() }
}

/// Adds a directory name that will be skipped during directory
/// linking. Not a path.
///
/// Any directory matching the name will be
/// skipped, regardless of it's path.
///
/// For example if .git is added, any sub-path or root-path
/// containing a .git directory will have the .git directly
/// skipped during directory linking.
pub fn add_skip_directory(directory: &str) {
    unsafe { usvfsAddSkipDirectory(widen_mut!(directory)) }
}

/// clears the directory skip-list
pub fn clear_skip_directories() {
    unsafe { usvfsClearSkipDirectories() }
}

/// adds a library to be force loaded when the given process is injected
pub fn force_load_library(processName: &str, libraryPath: &str) {
    unsafe { usvfsForceLoadLibrary(widen_mut!(processName), widen_mut!(libraryPath)) }
}

/// clears all previous calls to force_load_library()
pub fn clear_library_force_loads() {
    unsafe { usvfsClearLibraryForceLoads() }
}

/// print debugging info about the vfs. The format is currently not
/// fixed and may change between usvfs versions
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
    fn usvfsSetInstanceName(p: *mut Parameters, name: *const i8);
    fn usvfsSetDebugMode(p: *mut Parameters, debugMode: bool);
    fn usvfsSetLogLevel(p: *mut Parameters, level: LogLevel);
    fn usvfsSetCrashDumpType(p: *mut Parameters, dumpType: CrashDumpsType);
    fn usvfsSetCrashDumpPath(p: *mut Parameters, path: *const i8);
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
    fn rawBindings() {
        unsafe {
            let testParams = usvfsCreateParameters();
            usvfsFreeParameters(testParams);
        }
    }

    #[test]
    fn rawName() {
        unsafe {
            let p = usvfsCreateParameters();
            let name = CString::new("").expect("Cstring");
            usvfsSetInstanceName(p, name.as_ptr());
            usvfsFreeParameters(p);
        }
    }

    #[test]
    fn rawMode() {
        unsafe {
            let p = usvfsCreateParameters();
            usvfsSetDebugMode(p, false);
            usvfsFreeParameters(p);
        }
    }

    #[test]
    fn rawLevel() {
        unsafe {
            let p = usvfsCreateParameters();
            usvfsSetLogLevel(p, LogLevel::Debug);
            usvfsFreeParameters(p);
        }
    }

    #[test]
    fn rawType() {
        unsafe {
            let p = usvfsCreateParameters();
            usvfsSetCrashDumpType(p, CrashDumpsType::Full);
            usvfsFreeParameters(p);
        }
    }

    #[test]
    fn rawPath() {
        unsafe {
            let p = usvfsCreateParameters();
            let path = CString::new("").expect("CString failed");
            usvfsSetCrashDumpPath(p, path.as_ptr());
            usvfsFreeParameters(p);
        }
    }

    #[test]
    fn rawDelay() {
        unsafe {
            let p = usvfsCreateParameters();
            usvfsSetProcessDelay(p, 5);
            usvfsFreeParameters(p);
        }
    }

    #[test]
    fn stringRepr() {
        let debug = LogLevel::Debug;
        let info = LogLevel::Info;
        let warning = LogLevel::Warning;
        let error = LogLevel::Error;

        let nil = CrashDumpsType::Nil;
        let mini = CrashDumpsType::Mini;
        let data = CrashDumpsType::Data;
        let full = CrashDumpsType::Full;

        assert_eq!(debug.to_string(), "debug");
        assert_eq!(info.to_string(), "info");
        assert_eq!(warning.to_string(), "warning");
        assert_eq!(error.to_string(), "error");

        assert_eq!(nil.to_string(), "none");
        assert_eq!(mini.to_string(), "mini");
        assert_eq!(data.to_string(), "data");
        assert_eq!(full.to_string(), "full");
    }

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
