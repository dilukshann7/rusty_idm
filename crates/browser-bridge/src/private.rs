//! Current-user encryption for persisted private policy values; no UI prompts.
#[cfg(windows)]
pub fn crypt(input: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows::{
        core::PCWSTR,
        Win32::{
            Foundation::{LocalFree, HLOCAL},
            Security::Cryptography::*,
        },
    };
    let source = CRYPT_INTEGER_BLOB {
        cbData: input
            .len()
            .try_into()
            .map_err(|_| "Private settings are too large.")?,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        let result = if encrypt {
            CryptProtectData(
                &source,
                PCWSTR::null(),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &source,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        result.map_err(|_| {
            "Windows could not protect/read private settings for this user.".to_owned()
        })?;
        let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        std::ptr::write_bytes(output.pbData, 0, output.cbData as usize);
        let _ = LocalFree(Some(HLOCAL(output.pbData.cast())));
        Ok(bytes)
    }
}
#[cfg(not(windows))]
pub fn crypt(_: &[u8], _: bool) -> Result<Vec<u8>, String> {
    Err("Private settings require Windows current-user encryption.".into())
}
