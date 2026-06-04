/// Token 相关：DPAPI 加密 / 解密、注册表写入、登录逻辑

// 与 D2Loader.ps1 保持完全一致的 Entropy
const ENTROPY: [u8; 16] = [
    0xc8, 0x76, 0xf4, 0xae, 0x4c, 0x95, 0x2e, 0xfe,
    0xf2, 0xfa, 0x0f, 0x54, 0x19, 0xc0, 0x9c, 0x43,
];

/// 直接声明 DPAPI FFI，避免 windows crate 版本差异
#[cfg(windows)]
#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[cfg(windows)]
extern "system" {
    #[link_name = "CryptProtectData"]
    fn crypt_protect_data(
        p_data_in: *const DataBlob,
        sz_data_descr: *const u16,
        p_optional_entropy: *const DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *const core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;

    #[link_name = "CryptUnprotectData"]
    fn crypt_unprotect_data(
        p_data_in: *const DataBlob,
        ppsz_data_descr: *mut *mut u16,
        p_optional_entropy: *const DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *const core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;

    #[link_name = "LocalFree"]
    fn local_free(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

/// 用 Windows DPAPI (CurrentUser scope) 加密 token bytes
pub fn dpapi_encrypt_token(plain_token: &str) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    {
        let mut token_bytes = plain_token.as_bytes().to_vec();
        let mut entropy_copy = ENTROPY;
        let entropy_blob = DataBlob {
            cb_data: entropy_copy.len() as u32,
            pb_data: entropy_copy.as_mut_ptr(),
        };
        let input_blob = DataBlob {
            cb_data: token_bytes.len() as u32,
            pb_data: token_bytes.as_mut_ptr(),
        };
        let mut output_blob = DataBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };

        let ok = unsafe {
            crypt_protect_data(
                &input_blob,
                std::ptr::null(),
                &entropy_blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                &mut output_blob,
            )
        };

        if ok == 0 {
            return Err(format!(
                "CryptProtectData failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let data = unsafe {
            let len = output_blob.cb_data as usize;
            let d = std::slice::from_raw_parts(output_blob.pb_data, len).to_vec();
            local_free(output_blob.pb_data as *mut _);
            d
        };
        Ok(data)
    }
    #[cfg(not(windows))]
    {
        Err("DPAPI is only available on Windows".to_string())
    }
}

/// 用 Windows DPAPI 解密 token bytes → 明文 token string
pub fn dpapi_decrypt_token(encrypted: &[u8]) -> Result<String, String> {
    #[cfg(windows)]
    {
        let mut enc_copy = encrypted.to_vec();
        let mut entropy_copy = ENTROPY;
        let entropy_blob = DataBlob {
            cb_data: entropy_copy.len() as u32,
            pb_data: entropy_copy.as_mut_ptr(),
        };
        let input_blob = DataBlob {
            cb_data: enc_copy.len() as u32,
            pb_data: enc_copy.as_mut_ptr(),
        };
        let mut output_blob = DataBlob {
            cb_data: 0,
            pb_data: std::ptr::null_mut(),
        };
        let mut desc_ptr: *mut u16 = std::ptr::null_mut();

        let ok = unsafe {
            crypt_unprotect_data(
                &input_blob,
                &mut desc_ptr,
                &entropy_blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                &mut output_blob,
            )
        };

        if ok == 0 {
            return Err(format!(
                "CryptUnprotectData failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let data = unsafe {
            let len = output_blob.cb_data as usize;
            let d = std::slice::from_raw_parts(output_blob.pb_data, len).to_vec();
            local_free(output_blob.pb_data as *mut _);
            d
        };
        String::from_utf8(data).map_err(|e| e.to_string())
    }
    #[cfg(not(windows))]
    {
        Err("DPAPI is only available on Windows".to_string())
    }
}

/// 将 token 加密后写入注册表，供 D2R.exe 读取
#[tauri::command]
pub fn write_token_to_registry(plain_token: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use winreg::{enums::*, RegKey};

        let protected = dpapi_encrypt_token(&plain_token)?;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let reg_path = r"SOFTWARE\Blizzard Entertainment\Battle.net\Launch Options\OSI";
        let (key, _) = hkcu.create_subkey(reg_path).map_err(|e| e.to_string())?;

        key.set_value("REGION", &"CN").map_err(|e| e.to_string())?;
        key.set_raw_value(
            "WEB_TOKEN",
            &winreg::RegValue {
                bytes: protected,
                vtype: RegType::REG_BINARY,
            },
        )
        .map_err(|e| e.to_string())?;

        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Registry operations are only available on Windows".to_string())
    }
}

/// 打开登录窗口，拦截 localhost:0 回调并提取 ST token
#[tauri::command]
pub fn open_login_window(app: tauri::AppHandle, account_id: String) -> Result<(), String> {
    use tauri::{Emitter, Manager, WebviewWindowBuilder, WebviewUrl};
    
    // 如果已有登录窗口则关闭旧的
    if let Some(win) = app.get_webview_window("login") {
        let _ = win.close();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let login_url = "https://account.battlenet.com.cn/login/zh/?externalChallenge=login&app=OSI";
    let app_clone = app.clone();
    let aid = account_id.clone();

    // 创建登录窗口
    WebviewWindowBuilder::new(
        &app,
        "login",
        WebviewUrl::External(login_url.parse().map_err(|e: url::ParseError| e.to_string())?),
    )
    .title("Battle.net CN 登录")
    .inner_size(960.0, 720.0)
    .min_inner_size(800.0, 600.0)
    .center()
    .resizable(true)
    .visible(true)
    .focused(true)
    .incognito(true)
    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    .on_navigation(move |url| {
        // 拦截重定向到 http://localhost:0/?ST=...
        if let Some(host) = url.host_str() {
            if host == "localhost" {
                // 解析 ST 参数
                if let Some(token) = url
                    .query_pairs()
                    .find(|(k, _)| k == "ST")
                    .map(|(_, v)| v.into_owned())
                {
                    if !token.is_empty() {
                        // 发送事件给前端
                        let _ = app_clone.emit(
                            "token_captured",
                            serde_json::json!({ "account_id": aid.clone(), "token": token }),
                        );
                        // 关闭登录窗口
                        if let Some(win) = app_clone.get_webview_window("login") {
                            let _ = win.close();
                        }
                    }
                }
                // 阻止导航到无效地址
                return false;
            }
        }
        // 允许其他导航
        true
    })
    .build()
    .map_err(|e| format!("Failed to create login window: {}", e))?;

    Ok(())
}
