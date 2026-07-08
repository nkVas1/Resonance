//! Minimal NVAPI DRS bindings via nvapi_QueryInterface.
//!
//! Interface IDs cross-checked against nvidiaProfileInspector's NvapiDrsWrapper.cs
//! (primary/fallback pairs). DSR setting IDs are intentionally NOT hardcoded:
//! they are discovered at runtime via EnumAvailableSettingIds + GetSettingNameFromId.

use std::ffi::c_void;

pub type NvStatus = i32;
pub const NVAPI_OK: NvStatus = 0;

pub type DrsSession = *mut c_void;
pub type DrsProfile = *mut c_void;

pub const NVDRS_DWORD_TYPE: u32 = 0;

/// NVDRS_SETTING_V1 — layout mirrors nvapi.h exactly.
/// Unions are represented as raw byte blobs (largest member: NVDRS_BINARY_SETTING = 4+4096).
#[repr(C)]
pub struct NvdrsSetting {
    pub version: u32,
    pub setting_name: [u16; 2048],
    pub setting_id: u32,
    pub setting_type: u32,
    pub setting_location: u32,
    pub is_current_predefined: u32,
    pub is_predefined_valid: u32,
    pub predefined: [u8; 4100],
    pub current: [u8; 4100],
}

/// NVDRS_SETTING_VALUES — default + up to 100 possible values.
#[repr(C)]
pub struct NvdrsSettingValues {
    pub version: u32,
    pub num_setting_values: u32,
    pub setting_type: u32,
    pub default_value: [u8; 4100],
    pub setting_values: [[u8; 4100]; 100],
}

pub const fn nvapi_version<T>() -> u32 {
    (std::mem::size_of::<T>() as u32) | (1 << 16)
}

const _: () = assert!(std::mem::size_of::<NvdrsSetting>() == 12320);
const _: () = assert!(std::mem::size_of::<NvdrsSettingValues>() == 414112);

pub fn u32_from_blob(blob: &[u8]) -> u32 {
    u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]])
}

/// Heap-allocate a zeroed T without materializing it on the stack first
/// (NvdrsSettingValues is ~414 KB; `Box::new(zeroed())` overflows the stack in debug builds).
fn boxed_zeroed<T>() -> Box<T> {
    unsafe {
        let layout = std::alloc::Layout::new::<T>();
        let ptr = std::alloc::alloc_zeroed(layout) as *mut T;
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Box::from_raw(ptr)
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
}

type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *mut c_void;

macro_rules! nv_call {
    ($api:expr, $fn:expr, $($arg:expr),*) => {{
        let status = unsafe { ($fn)($($arg),*) };
        if status != NVAPI_OK {
            Err(format!("{} failed: {} ({})", stringify!($fn), $api.error_message(status), status))
        } else {
            Ok(())
        }
    }};
}

pub struct NvApi {
    initialize: unsafe extern "C" fn() -> NvStatus,
    unload: unsafe extern "C" fn() -> NvStatus,
    get_error_message: unsafe extern "C" fn(NvStatus, *mut [u8; 64]) -> NvStatus,
    drs_create_session: unsafe extern "C" fn(*mut DrsSession) -> NvStatus,
    drs_destroy_session: unsafe extern "C" fn(DrsSession) -> NvStatus,
    drs_load_settings: unsafe extern "C" fn(DrsSession) -> NvStatus,
    drs_save_settings: unsafe extern "C" fn(DrsSession) -> NvStatus,
    drs_get_base_profile: unsafe extern "C" fn(DrsSession, *mut DrsProfile) -> NvStatus,
    drs_get_setting: unsafe extern "C" fn(DrsSession, DrsProfile, u32, *mut NvdrsSetting) -> NvStatus,
    drs_set_setting: unsafe extern "C" fn(DrsSession, DrsProfile, *mut NvdrsSetting) -> NvStatus,
    drs_enum_available_setting_ids: unsafe extern "C" fn(*mut u32, *mut u32) -> NvStatus,
    drs_get_setting_name_from_id: unsafe extern "C" fn(u32, *mut [u16; 2048]) -> NvStatus,
    drs_enum_available_setting_values:
        unsafe extern "C" fn(u32, *mut u32, *mut NvdrsSettingValues) -> NvStatus,
}

impl NvApi {
    pub fn load() -> Result<Self, String> {
        let dll: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
        let module = unsafe { LoadLibraryW(dll.as_ptr()) };
        if module.is_null() {
            return Err("nvapi64.dll not found — NVIDIA driver missing?".into());
        }
        let qi = unsafe { GetProcAddress(module, b"nvapi_QueryInterface\0".as_ptr()) };
        if qi.is_null() {
            return Err("nvapi_QueryInterface export missing".into());
        }
        let qi: QueryInterfaceFn = unsafe { std::mem::transmute(qi) };

        let resolve = |name: &str, primary: u32, fallback: u32| -> Result<*mut c_void, String> {
            let mut ptr = unsafe { qi(primary) };
            if ptr.is_null() && fallback != 0 {
                ptr = unsafe { qi(fallback) };
            }
            if ptr.is_null() {
                Err(format!("NVAPI function {name} (0x{primary:08X}) unavailable in this driver"))
            } else {
                Ok(ptr)
            }
        };

        macro_rules! get {
            ($name:literal, $primary:expr, $fallback:expr) => {
                unsafe { std::mem::transmute(resolve($name, $primary, $fallback)?) }
            };
        }

        let api = NvApi {
            initialize: get!("Initialize", 0x0150E828, 0),
            unload: get!("Unload", 0xD22BDD7E, 0),
            get_error_message: get!("GetErrorMessage", 0x6C2D048C, 0),
            drs_create_session: get!("DRS_CreateSession", 0x0694D52E, 0),
            drs_destroy_session: get!("DRS_DestroySession", 0xDAD9CFF8, 0),
            drs_load_settings: get!("DRS_LoadSettings", 0x375DBD6B, 0),
            drs_save_settings: get!("DRS_SaveSettings", 0xFCBC7E14, 0),
            drs_get_base_profile: get!("DRS_GetBaseProfile", 0xDA8466A0, 0),
            drs_get_setting: get!("DRS_GetSetting", 0x73BF8338, 0xEA99498D),
            drs_set_setting: get!("DRS_SetSetting", 0x577DD202, 0x8A2CF5F5),
            drs_enum_available_setting_ids: get!("DRS_EnumAvailableSettingIds", 0xF020614A, 0xE5DE48E5),
            drs_get_setting_name_from_id: get!("DRS_GetSettingNameFromId", 0xD61CBE6E, 0x1EB13791),
            drs_enum_available_setting_values: get!("DRS_EnumAvailableSettingValues", 0x2EC39F90, 0),
        };

        let status = unsafe { (api.initialize)() };
        if status != NVAPI_OK {
            return Err(format!("NvAPI_Initialize failed: {}", api.error_message(status)));
        }
        Ok(api)
    }

    pub fn error_message(&self, status: NvStatus) -> String {
        let mut buf = [0u8; 64];
        unsafe { (self.get_error_message)(status, &mut buf) };
        let len = buf.iter().position(|&b| b == 0).unwrap_or(64);
        String::from_utf8_lossy(&buf[..len]).into_owned()
    }

    pub fn with_base_profile<T>(
        &self,
        f: impl FnOnce(&Self, DrsSession, DrsProfile) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut session: DrsSession = std::ptr::null_mut();
        nv_call!(self, self.drs_create_session, &mut session)?;
        let result = (|| {
            nv_call!(self, self.drs_load_settings, session)?;
            let mut profile: DrsProfile = std::ptr::null_mut();
            nv_call!(self, self.drs_get_base_profile, session, &mut profile)?;
            f(self, session, profile)
        })();
        unsafe { (self.drs_destroy_session)(session) };
        result
    }

    /// All setting IDs the driver knows about, with their driver-reported names.
    pub fn available_settings(&self) -> Result<Vec<(u32, String)>, String> {
        let mut ids = vec![0u32; 2048];
        let mut count = ids.len() as u32;
        nv_call!(self, self.drs_enum_available_setting_ids, ids.as_mut_ptr(), &mut count)?;
        ids.truncate(count as usize);

        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            let mut name = Box::new([0u16; 2048]);
            let status = unsafe { (self.drs_get_setting_name_from_id)(id, &mut *name) };
            let text = if status == NVAPI_OK {
                let len = name.iter().position(|&c| c == 0).unwrap_or(0);
                String::from_utf16_lossy(&name[..len])
            } else {
                String::new()
            };
            out.push((id, text));
        }
        out.sort_by_key(|(id, _)| *id);
        Ok(out)
    }

    /// Driver-declared possible values for a DWORD setting: (type, default, values).
    pub fn setting_values(&self, id: u32) -> Result<(u32, u32, Vec<u32>), String> {
        let mut values: Box<NvdrsSettingValues> = boxed_zeroed();
        values.version = nvapi_version::<NvdrsSettingValues>();
        let mut max = 100u32;
        nv_call!(self, self.drs_enum_available_setting_values, id, &mut max, &mut *values)?;
        let n = (values.num_setting_values as usize).min(100);
        let list = values.setting_values[..n].iter().map(|b| u32_from_blob(b)).collect();
        Ok((values.setting_type, u32_from_blob(&values.default_value), list))
    }

    /// Current DWORD value of a setting in the given profile, if present.
    pub fn get_dword(
        &self,
        session: DrsSession,
        profile: DrsProfile,
        id: u32,
    ) -> Result<Option<u32>, String> {
        let mut setting: Box<NvdrsSetting> = boxed_zeroed();
        setting.version = nvapi_version::<NvdrsSetting>();
        let status = unsafe { (self.drs_get_setting)(session, profile, id, &mut *setting) };
        match status {
            NVAPI_OK => Ok(Some(u32_from_blob(&setting.current))),
            -160 => Ok(None), // NVAPI_SETTING_NOT_FOUND
            other => Err(format!("DRS_GetSetting(0x{id:08X}): {}", self.error_message(other))),
        }
    }

    pub fn set_dword(
        &self,
        session: DrsSession,
        profile: DrsProfile,
        id: u32,
        value: u32,
    ) -> Result<(), String> {
        let mut setting: Box<NvdrsSetting> = boxed_zeroed();
        setting.version = nvapi_version::<NvdrsSetting>();
        setting.setting_id = id;
        setting.setting_type = NVDRS_DWORD_TYPE;
        setting.current[..4].copy_from_slice(&value.to_le_bytes());
        nv_call!(self, self.drs_set_setting, session, profile, &mut *setting)?;
        nv_call!(self, self.drs_save_settings, session)
    }
}

impl Drop for NvApi {
    fn drop(&mut self) {
        unsafe { (self.unload)() };
    }
}
