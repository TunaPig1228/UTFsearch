use std::path::Path;

/// Best-effort OS file owner (修改者). Empty when unavailable (common on some NAS).
pub fn file_owner(path: &Path) -> Option<String> {
    #[cfg(unix)]
    {
        unix_owner(path)
    }
    #[cfg(windows)]
    {
        windows_owner(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

#[cfg(unix)]
fn unix_owner(path: &Path) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path).ok()?;
    Some(meta.uid().to_string())
}

#[cfg(windows)]
fn windows_owner(path: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{LocalFree, ERROR_SUCCESS};
    use windows_sys::Win32::Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        LookupAccountSidW, OWNER_SECURITY_INFORMATION, SID_NAME_USE,
    };

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut psid: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut sd: *mut core::ffi::c_void = std::ptr::null_mut();
    // Safety: Win32 GetNamedSecurityInfoW; LocalFree the returned descriptor.
    let err = unsafe {
        GetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            &mut psid,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut sd,
        )
    };
    if err != ERROR_SUCCESS || psid.is_null() {
        if !sd.is_null() {
            unsafe {
                LocalFree(sd as _);
            }
        }
        return None;
    }

    let mut name = [0u16; 256];
    let mut domain = [0u16; 256];
    let mut name_n = name.len() as u32;
    let mut domain_n = domain.len() as u32;
    let mut use_ty: SID_NAME_USE = 0;
    let ok = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            psid,
            name.as_mut_ptr(),
            &mut name_n,
            domain.as_mut_ptr(),
            &mut domain_n,
            &mut use_ty,
        )
    };
    if !sd.is_null() {
        unsafe {
            LocalFree(sd as _);
        }
    }
    if ok == 0 {
        return None;
    }
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    let s = String::from_utf16_lossy(&name[..end]);
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
