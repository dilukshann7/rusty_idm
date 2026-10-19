use std::{io, mem::size_of};
use windows::{
    core::{PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL},
        Security::{Authorization::*, *},
        System::{Pipes::*, RemoteDesktop::ProcessIdToSessionId, Threading::*},
    },
};
pub struct OwnedHandle(pub HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserSession {
    pub user_sid: String,
    pub logon_sid: String,
    pub session_id: u32,
}
fn error(e: windows::core::Error) -> io::Error {
    io::Error::other(e)
}
fn token_data(token: HANDLE, class: TOKEN_INFORMATION_CLASS) -> io::Result<Vec<usize>> {
    let mut size = 0;
    unsafe {
        let _ = GetTokenInformation(token, class, None, 0, &mut size);
    }
    if size == 0 || size > 65536 {
        return Err(io::Error::other("Invalid token size"));
    }
    let mut data = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
    unsafe {
        GetTokenInformation(
            token,
            class,
            Some(data.as_mut_ptr().cast()),
            size,
            &mut size,
        )
        .map_err(error)?;
    }
    Ok(data)
}
fn sid_text(sid: PSID) -> io::Result<String> {
    unsafe {
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(sid, &mut text).map_err(error)?;
        let result = text.to_string().map_err(error);
        LocalFree(Some(HLOCAL(text.0.cast())));
        result
    }
}
impl UserSession {
    pub fn current() -> io::Result<Self> {
        unsafe { Self::from_process(GetCurrentProcessId()) }
    }
    unsafe fn from_process(pid: u32) -> io::Result<Self> {
        unsafe {
            let process = OwnedHandle(
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).map_err(error)?,
            );
            let mut token = HANDLE::default();
            OpenProcessToken(process.0, TOKEN_QUERY, &mut token).map_err(error)?;
            let token = OwnedHandle(token);
            let mut session_id = 0;
            ProcessIdToSessionId(pid, &mut session_id).map_err(error)?;
            Self::from_token(token.0, session_id)
        }
    }
    fn from_token(token: HANDLE, session_id: u32) -> io::Result<Self> {
        let user = token_data(token, TokenUser)?;
        let groups = token_data(token, TokenGroups)?;
        unsafe {
            let user_sid = sid_text((*(user.as_ptr() as *const TOKEN_USER)).User.Sid)?;
            let groups = &*(groups.as_ptr() as *const TOKEN_GROUPS);
            let list =
                std::slice::from_raw_parts(groups.Groups.as_ptr(), groups.GroupCount as usize);
            let logon = list
                .iter()
                .find(|g| g.Attributes & 0xc0000000 == 0xc0000000)
                .ok_or_else(|| io::Error::other("Missing logon identity"))?;
            Ok(Self {
                user_sid,
                logon_sid: sid_text(logon.Sid)?,
                session_id,
            })
        }
    }
    pub fn same_identity(&self, other: &Self) -> bool {
        self == other
    }
    pub fn pipe_name(&self) -> String {
        format!(
            r"\\.\pipe\RustyIDM.Browser.v1.{}.{}",
            self.user_sid, self.session_id
        )
    }
    pub fn mutex_name(&self) -> String {
        format!(
            r"Local\RustyIDM.Browser.v1.{}.{}",
            self.user_sid, self.session_id
        )
    }
    pub fn verify_server(&self, handle: HANDLE) -> io::Result<()> {
        unsafe {
            let mut pid = 0;
            GetNamedPipeServerProcessId(handle, &mut pid).map_err(error)?;
            if !self.same_identity(&Self::from_process(pid)?) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Unexpected pipe server",
                ));
            }
            Ok(())
        }
    }
    pub fn verify_client(&self, handle: HANDLE) -> io::Result<()> {
        unsafe {
            let mut pid = 0;
            GetNamedPipeClientProcessId(handle, &mut pid).map_err(error)?;
            if !self.same_identity(&Self::from_process(pid)?) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Unexpected pipe client",
                ));
            }
            ImpersonateNamedPipeClient(handle).map_err(error)?;
            struct Revert;
            impl Drop for Revert {
                fn drop(&mut self) {
                    unsafe {
                        if RevertToSelf().is_err() {
                            std::process::abort();
                        }
                    }
                }
            }
            let _revert = Revert;
            let mut token = HANDLE::default();
            OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token).map_err(error)?;
            let token = OwnedHandle(token);
            if !self.same_identity(&Self::from_token(token.0, self.session_id)?) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "Unexpected impersonated identity",
                ));
            }
            Ok(())
        }
    }
}
pub struct SecurityDescriptor(PSECURITY_DESCRIPTOR);
impl SecurityDescriptor {
    pub fn new(session: &UserSession) -> io::Result<Self> {
        let text: Vec<u16> = format!("D:P(A;;GA;;;{})", session.logon_sid)
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR(text.as_ptr()),
                1,
                &mut descriptor,
                None,
            )
            .map_err(error)?;
        }
        Ok(Self(descriptor))
    }
    pub fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.0 .0,
            bInheritHandle: false.into(),
        }
    }
}
impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        unsafe {
            LocalFree(Some(HLOCAL(self.0 .0)));
        }
    }
}
