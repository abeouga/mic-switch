#![allow(non_snake_case)]

use std::ffi::c_void;

use windows::core::{interface, IUnknown, IUnknown_Vtbl, GUID, HRESULT, PCWSTR, PWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eCapture, eCommunications, eConsole, eMultimedia, eRender, EDataFlow, IMMDevice,
    IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
};

pub type Result<T> = windows::core::Result<T>;

#[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
unsafe trait IPolicyConfig: IUnknown {
    fn GetMixFormat(&self, id: PCWSTR, format: *mut *mut c_void) -> HRESULT;
    fn GetDeviceFormat(&self, id: PCWSTR, default: i32, format: *mut *mut c_void) -> HRESULT;
    fn ResetDeviceFormat(&self, id: PCWSTR) -> HRESULT;
    fn SetDeviceFormat(&self, id: PCWSTR, endpoint: *mut c_void, mix: *mut c_void) -> HRESULT;
    fn GetProcessingPeriod(&self, id: PCWSTR, default: i32, def: *mut i64, min: *mut i64) -> HRESULT;
    fn SetProcessingPeriod(&self, id: PCWSTR, period: *mut i64) -> HRESULT;
    fn GetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn SetShareMode(&self, id: PCWSTR, mode: *mut c_void) -> HRESULT;
    fn GetPropertyValue(&self, id: PCWSTR, fx: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
    fn SetPropertyValue(&self, id: PCWSTR, fx: i32, key: *const c_void, value: *mut c_void) -> HRESULT;
    fn SetDefaultEndpoint(&self, id: PCWSTR, role: i32) -> HRESULT;
    fn SetEndpointVisibility(&self, id: PCWSTR, visible: i32) -> HRESULT;
}

const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Input,
    Output,
}

impl Flow {
    pub const ALL: [Flow; 2] = [Flow::Input, Flow::Output];

    fn data_flow(self) -> EDataFlow {
        match self {
            Flow::Input => eCapture,
            Flow::Output => eRender,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn com_init() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn enumerator() -> Result<IMMDeviceEnumerator> {
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

unsafe fn take_string(p: PWSTR) -> String {
    let s = p.to_string().unwrap_or_default();
    CoTaskMemFree(Some(p.0 as *const c_void));
    s
}

fn device_id(device: &IMMDevice) -> Result<String> {
    unsafe { Ok(take_string(device.GetId()?)) }
}

fn device_name(device: &IMMDevice) -> String {
    unsafe {
        device
            .OpenPropertyStore(STGM_READ)
            .and_then(|store| store.GetValue(&PKEY_Device_FriendlyName))
            .map(|value| value.to_string())
            .unwrap_or_default()
    }
}

pub fn default_id(flow: Flow) -> Option<String> {
    unsafe {
        let device = enumerator().ok()?.GetDefaultAudioEndpoint(flow.data_flow(), eConsole).ok()?;
        device_id(&device).ok()
    }
}

pub fn list(flow: Flow) -> Result<Vec<Device>> {
    let default = default_id(flow);
    unsafe {
        let collection = enumerator()?.EnumAudioEndpoints(flow.data_flow(), DEVICE_STATE_ACTIVE)?;
        let mut devices = Vec::new();
        for i in 0..collection.GetCount()? {
            let device = collection.Item(i)?;
            let id = device_id(&device)?;
            devices.push(Device {
                is_default: default.as_deref() == Some(id.as_str()),
                name: device_name(&device),
                id,
            });
        }
        Ok(devices)
    }
}

pub fn set_default(id: &str) -> Result<()> {
    let id = wide(id);
    unsafe {
        let policy: IPolicyConfig = CoCreateInstance(&CLSID_POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)?;
        for role in [eConsole, eMultimedia, eCommunications] {
            policy.SetDefaultEndpoint(PCWSTR(id.as_ptr()), role.0).ok()?;
        }
    }
    Ok(())
}

fn endpoint_volume(id: &str) -> Result<IAudioEndpointVolume> {
    let id = wide(id);
    unsafe { enumerator()?.GetDevice(PCWSTR(id.as_ptr()))?.Activate(CLSCTX_ALL, None) }
}

pub fn volume(id: &str) -> Result<(f32, bool)> {
    unsafe {
        let endpoint = endpoint_volume(id)?;
        Ok((endpoint.GetMasterVolumeLevelScalar()?, endpoint.GetMute()?.as_bool()))
    }
}

pub fn set_volume(id: &str, level: f32) -> Result<()> {
    unsafe { endpoint_volume(id)?.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null()) }
}

pub fn set_mute(id: &str, muted: bool) -> Result<()> {
    unsafe { endpoint_volume(id)?.SetMute(muted, std::ptr::null()) }
}
